{
  description = "Krezh's NixOS Flake";

  nixConfig = {
    extra-trusted-substituters = [
      "https://nix-cache.plexuz.xyz/krezh"
      "https://krezh.cachix.org"
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "krezh:bCYQVVbREhrYgC42zUMf99dMtVXIATXMCcq+wRimqCc="
      "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    hardware.url = "github:nixos/nixos-hardware";
    chaotic.url = "github:chaotic-cx/nyx/nyxpkgs-unstable";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts.url = "github:hercules-ci/flake-parts";

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

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    catppuccin = {
      url = "github:catppuccin/nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-index = {
      url = "github:nix-community/nix-index-database";
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

    nix-gaming.url = "github:fufexan/nix-gaming";

    elephant = {
      url = "github:abenz1267/elephant";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    walker = {
      url = "github:abenz1267/walker";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.elephant.follows = "elephant";
    };

    cache-nix-action = {
      url = "github:nix-community/cache-nix-action";
      flake = false;
    };

    kauth = {
      url = "github:krezh/kauth";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    gomod2nix = {
      url = "github:nix-community/gomod2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    caelestia-shell = {
      url = "github:caelestia-dots/shell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    dgop = {
      url = "github:AvengeMedia/dgop";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    dms-cli = {
      url = "github:AvengeMedia/danklinux";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    dankMaterialShell = {
      url = "github:AvengeMedia/DankMaterialShell";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.dgop.follows = "dgop";
      inputs.dms-cli.follows = "dms-cli";
    };
  };

  outputs =
    inputs@{ flake-parts, self, ... }:
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
        ghMatrix = flakeLib.ghMatrix { exclude = [ "nixos-livecd" ]; };
        top = flakeLib.top;

        overlays = import ./overlays { inherit inputs lib; };
        homeManagerModules = [ ./modules/homeManager ];
        nixosModules.default.imports = [ ./modules/nixos ];
      };

      perSystem =
        { pkgs, config, ... }:
        {
          pre-commit = import ./pre-commit.nix { inherit pkgs; };
          devshells = import ./shell.nix {
            inherit
              pkgs
              config
              lib
              ;
          };
          packages = (import ./pkgs { inherit pkgs lib; }) // {
            # this is for cache-nix-action so stuff don't get garbage collected
            # before I cache the nix store, mostly to not redownload inputs a lot
            gc-keep =
              (import "${inputs.cache-nix-action}/saveFromGC.nix" {
                inherit pkgs inputs;
                inputsExclude = [
                  inputs.flake-parts
                ];
              }).saveFromGC;
          };
          treefmt = import ./treefmt.nix { inherit pkgs; };
        };
    };
}
