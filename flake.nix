{
  description = "Krezh Nix Flake";

  nixConfig = {};

  inputs = {
    # Nixpkgs
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    impermanence.url = "github:nix-community/impermanence";

    # Home Manager
    home-manager = {
      url = "github:nix-community/home-manager/release-23.11";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # nix-fast-build
    nix-fast-build = {
      url = "github:Mic92/nix-fast-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nh = {
      url = "github:viperml/nh";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprland = {
      url = "github:hyprwm/hyprland";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    hyprwm-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprland-plugins = {
      url = "github:hyprwm/hyprland-plugins";
      inputs.hyprland.follows = "hyprland";
    };

    # Hardware
    hardware.url = "github:nixos/nixos-hardware";

    # sops-nix
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixd.url = "github:nix-community/nixd";

    # NixOS-WSL
    nixos-wsl = {
      url = "github:nix-community/NixOS-WSL";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-compat.follows = "";
      };
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    home-manager,
    ...
  }: let
    inherit (self) outputs;
    lib = nixpkgs.lib // home-manager.lib;
    forAllSystems = nixpkgs.lib.genAttrs [
      "aarch64-darwin"
      "aarch64-linux"
      "x86_64-darwin"
      "x86_64-linux"
    ];
  in {
    inherit lib;

    home-manager.sharedModules = [
      inputs.sops-nix.homeManagerModules.sops
    ];

    packages = forAllSystems (pkgs: import ./packages { inherit pkgs; });
    formatter = forAllSystems (pkgs: pkgs.nixpkgs-fmt);
    devShells = forAllSystems (pkgs: import ./shell.nix { inherit pkgs; });
    overlays = import ./overlays { inherit inputs outputs; };
    nixosModules = import ./modules/nixos;
    commonModules = import ./modules/common;
    homeManagerModules = import ./modules/home-manager;

    pkgs = forAllSystems (localSystem:
      import nixpkgs {
        inherit localSystem;
        overlays = [self.overlays.default];
        config = {
          allowUnfree = true;
          allowAliases = true;
        };
      });

    nixosConfigurations = {
      thor-wsl = nixpkgs.lib.nixosSystem {
        specialArgs = {inherit self inputs outputs;};
        modules = [./hosts/thor-wsl];
      };
      odin = nixpkgs.lib.nixosSystem {
        specialArgs = {inherit self inputs outputs;};
        modules = [./hosts/odin-wsl];
      };
    };
  };
}
