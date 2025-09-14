{
  description = "Krezh's NixOS Flake";

  nixConfig = {
    extra-trusted-substituters = [
      "https://nix-cache.plexuz.xyz/krezh"
      "https://krezh.cachix.org"
      "https://cache.garnix.io"
      "https://nix-community.cachix.org"
      "https://hyprland.cachix.org"
    ];
    extra-trusted-public-keys = [
      "krezh:bCYQVVbREhrYgC42zUMf99dMtVXIATXMCcq+wRimqCc="
      "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
      "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    hardware.url = "github:nixos/nixos-hardware";
    chaotic.url = "github:chaotic-cx/nyx/nyxpkgs-unstable";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
    };

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-ld = {
      url = "github:Mic92/nix-ld";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    catppuccin.url = "github:catppuccin/nix";
    catppuccin.inputs.nixpkgs.follows = "nixpkgs";

    nix-index = {
      url = "github:nix-community/nix-index-database";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprpanel = {
      url = "github:Jas-SinghFSU/HyprPanel";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # NeoVIM
    nvf = {
      url = "github:notashelf/nvf";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixos-wsl = {
      url = "github:nix-community/nixos-wsl";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    zen-browser-catppuccin = {
      url = "github:catppuccin/zen-browser";
      flake = false;
    };

    betterfox = {
      url = "github:yokoffing/Betterfox";
      flake = false;
    };

    sherlock = {
      url = "github:Skxxtz/sherlock";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    swww.url = "github:LGFae/swww";
    swww.inputs.nixpkgs.follows = "nixpkgs";

    nixcord.url = "github:kaylorben/nixcord";
    nixcord.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{
      flake-parts,
      self,
      ...
    }:
    let
      inherit (self) outputs;

      lib = inputs.nixpkgs.lib // import ./lib { inherit inputs; };

      flakeLib = import ./lib/flakeLib.nix {
        inherit
          inputs
          outputs
          lib
          self
          ;
      };
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.pre-commit-hooks.flakeModule
        inputs.devshell.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      systems = [ "x86_64-linux" ];

      flake = {
        nixosConfigurations = flakeLib.mkSystems [
          {
            hostname = "thor";
            system = "x86_64-linux";
            homeUsers = [ "krezh" ];
          }
          {
            hostname = "odin";
            system = "x86_64-linux";
            homeUsers = [ "krezh" ];
          }
          {
            hostname = "nixos-livecd";
            system = "x86_64-linux";
            homeUsers = [ ];
          }
        ];
        evalHosts = flakeLib.evalHosts;
        top = flakeLib.top;

        overlays = import ./overlays { inherit inputs lib; };
        homeManagerModules = [
          ./modules/homeManager
        ];
        nixosModules.default = {
          imports = [
            ./modules/nixos
          ];
        };
        om.ci.default.root.dir = ".";
      };

      perSystem =
        { pkgs, config, ... }:
        {
          pre-commit = import ./pre-commit.nix { inherit pkgs; };
          devshells = import ./shell.nix {
            inherit
              inputs
              pkgs
              config
              lib
              ;
          };
          packages = import ./pkgs { inherit pkgs lib; };
          treefmt = import ./treefmt.nix { inherit pkgs; };
        };
    };
}
