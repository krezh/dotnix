{
  description = "Krezh Nix Flake";

  nixConfig = {};

  inputs = {
    # Nixpkgs
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";

    # Home Manager
    home-manager = {
      url = "github:nix-community/home-manager/release-23.11";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # nix-fast-build
    nix-fast-build = {
      url = "github:Mic92/nix-fast-build";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
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
    forAllSystems = nixpkgs.lib.genAttrs [
      "aarch64-darwin"
      "aarch64-linux"
      "x86_64-darwin"
      "x86_64-linux"
    ];
  in {
    # Your custom packages
    # Accessible through 'nix build', 'nix shell', etc
    packages = forAllSystems (system: import ./packages nixpkgs.legacyPackages.${system});
    # Formatter for your nix files, available through 'nix fmt'
    # Other options beside 'alejandra' include 'nixpkgs-fmt'
    formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.alejandra);

    # Your custom packages and modifications, exported as overlays
    overlays = import ./overlays {inherit inputs;};
    # Reusable nixos modules you might want to export
    # These are usually stuff you would upstream into nixpkgs
    nixosModules = import ./modules/nixos;
    commonModules = import ./modules/common;
    # Reusable home-manager modules you might want to export
    # These are usually stuff you would upstream into home-manager
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

    # NixOS configuration entrypoint
    # Available through 'nixos-rebuild --flake .#your-hostname'
    nixosConfigurations = {
      # hostname
      nixos = nixpkgs.lib.nixosSystem {
        specialArgs = {inherit self inputs outputs;};
        # > Our main nixos configuration file <
        modules = [
          ./nixos/configuration.nix
        ];
      };
    };
  };
}
