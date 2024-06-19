{
  description = "Krezh Nix Flake";

  nixConfig = {
    extra-trusted-substituters = [
      "https://krezh.cachix.org"
      "https://nix-community.cachix.org"
      "https://hyprland.cachix.org"
    ];
    extra-trusted-public-keys = [
      "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    impermanence.url = "github:nix-community/impermanence";
    nur.url = "github:nix-community/NUR";
    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    hardware.url = "github:nixos/nixos-hardware";

    nixd.url = "github:nix-community/nixd";

    nix-ld.url = "github:Mic92/nix-ld";
    nix-ld.inputs.nixpkgs.follows = "nixpkgs";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-fast-build = {
      url = "github:Mic92/nix-fast-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nh = {
      url = "github:viperml/nh";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprland = {
      url = "git+https://github.com/hyprwm/Hyprland?submodules=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprland-plugins = {
      url = "github:hyprwm/hyprland-plugins";
      inputs.hyprland.follows = "hyprland";
    };

    hyprland-contrib = {
      url = "github:hyprwm/contrib";
    };

    hypridle = {
      url = "github:hyprwm/hypridle";
    };

    hyprlock = {
      url = "github:hyprwm/hyprlock";
    };

    hyprkeys = {
      url = "github:hyprland-community/hyprkeys";
    };

    xdg-portal-hyprland.url = "github:hyprwm/xdg-desktop-portal-hyprland";

    swww.url = "github:LGFae/swww";

    ags.url = "github:Aylur/ags";

    gBar.url = "github:scorpion-26/gBar";

    waybar.url = "github:Alexays/Waybar";

    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixvim = {
      url = "github:nix-community/nixvim";
      #inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprfocus = {
      url = "github:/VortexCoyote/hyprfocus";
    };

    deadnix = {
      url = "github:astro/deadnix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixos-wsl = {
      url = "github:nix-community/NixOS-WSL";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    nixos-wsl-vscode = {
      url = "github:K900/vscode-remote-workaround";
    };

    wezterm = {
      url = "github:wez/wezterm?dir=nix";
    };

    talosctl = {
      url = "github:szinn/nix-config";
    };

    catppuccin.url = "github:catppuccin/nix";
  };

  outputs =
    inputs@{ self, nixpkgs, ... }:
    let
      inherit (self) outputs;
      systems = [
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-linux"
        "x86_64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {

      packages = forAllSystems (pkgs: import ./pkgs { inherit pkgs; });

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-rfc-style);

      # devShells = forAllSystems (pkgs: import ./shell.nix { inherit pkgs; });

      overlays = import ./overlays { inherit inputs; };

      nixosModules = import ./modules/nixos;
      commonModules = import ./modules/common;
      homeManagerModules = import ./modules/home-manager;

      nixosConfigurations = {
        thor-wsl = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = {
            inherit self inputs outputs;
          };
          modules = [ ./hosts/thor-wsl ];
        };
        odin = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = {
            inherit self inputs outputs;
          };
          modules = [ ./hosts/odin ];
        };
      };
      # Convenience output that aggregates the outputs for home, nixos.
      # Also used in ci to build targets generally.
      top =
        let
          nixtop = nixpkgs.lib.genAttrs (builtins.attrNames inputs.self.nixosConfigurations) (
            attr: inputs.self.nixosConfigurations.${attr}.config.system.build.toplevel
          );
        in
        nixtop;
    };
}
