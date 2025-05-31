{
  description = "Krezh's NixOS Flake";

  nixConfig = {
    extra-trusted-substituters = [
      "https://krezh.cachix.org"
      "https://cache.garnix.io"
      "https://install.determinate.systems"
      "https://nix-community.cachix.org"
      "https://hyprland.cachix.org"
    ];
    extra-trusted-public-keys = [
      "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
      "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
      "cache.flakehub.com-3:hJuILl5sVK4iKm86JzgdXW12Y2Hwd5G07qKtHTOcDCM="
      "cache.lix.systems:aBnZUw8zA7H35Cz2RyKFVs3H4PlGTLawyY5KRbvJR8o="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    determinate.url = "https://flakehub.com/f/DeterminateSystems/determinate/*";

    home-manager = {
      url = "github:nix-community/home-manager/release-24.11";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
    };

    hardware.url = "github:nixos/nixos-hardware";

    nixos-facter-modules.url = "github:numtide/nixos-facter-modules";

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixos-generators = {
      url = "github:nix-community/nixos-generators";
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

    nix-index = {
      url = "github:nix-community/nix-index-database";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixd = {
      url = "github:nix-community/nixd";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    swww = {
      url = "github:LGFae/swww";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprpanel = {
      url = "github:Jas-SinghFSU/HyprPanel";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    ironbar = {
      url = "github:JakeStanger/ironbar";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nvf = {
      url = "github:notashelf/nvf";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixos-wsl = {
      url = "github:nix-community/NixOS-WSL";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    ghostty = {
      url = "github:ghostty-org/ghostty";
    };

    nix-github-actions.url = "github:nix-community/nix-github-actions";
    nix-github-actions.inputs.nixpkgs.follows = "nixpkgs";

    ssh-keys = {
      url = "https://github.com/krezh.keys";
      flake = false;
    };
  };

  outputs =
    inputs@{
      flake-parts,
      self,
      nix-github-actions,
      ...
    }:
    let
      inherit (self) outputs;

      lib = inputs.nixpkgs.lib // import ./lib { inherit inputs; };

      flakeLib = import ./lib/flakeLib.nix { inherit inputs outputs lib; };
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.pre-commit-hooks.flakeModule
        inputs.devshell.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

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
            hostname = "thor-wsl";
            system = "x86_64-linux";
            homeUsers = [ "krezh" ];
          }
          # {
          #   hostname = "rpi-01";
          #   system = "aarch64-linux";
          #   homeUsers = [ "krezh" ];
          # }
          # {
          #   hostname = "rpi-02";
          #   system = "aarch64-linux";
          #   homeUsers = [ "krezh" ];
          # }
          {
            hostname = "nixos-livecd";
            system = "x86_64-linux";
            homeUsers = [ ];
          }
        ];

        # Used by CI
        top = lib.genAttrs (builtins.attrNames self.nixosConfigurations) (
          attr: self.nixosConfigurations.${attr}.config.system.build.toplevel
        );

        # Lists hosts with their system kind for use in github actions
        evalHosts = {
          include = builtins.map (host: {
            inherit host;
            system = self.nixosConfigurations.${host}.pkgs.system;
            runner = lib.mapToGha self.nixosConfigurations.${host}.pkgs.system;
          }) (builtins.attrNames self.nixosConfigurations);
        };

        githubActions = nix-github-actions.lib.mkGithubMatrix { checks = self.packages; };

        overlays = import ./overlays { inherit inputs lib; };

        homeManagerModules = lib.scanPath.toList { path = ./modules/homeManager; };

        nixosModules.default = {
          imports = lib.scanPath.toList { path = ./modules/nixos; };
        };
      };

      perSystem =
        {
          pkgs,
          config,
          ...
        }:
        {
          pre-commit = import ./pre-commit.nix { inherit pkgs; };
          devshells = import ./shell.nix { inherit inputs pkgs config; };
          packages = import ./pkgs { inherit pkgs lib; };
          treefmt = {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = pkgs.lib.meta.availableOn pkgs.stdenv.buildPlatform pkgs.nixfmt-rfc-style.compiler;
            programs.nixfmt.package = pkgs.nixfmt-rfc-style;
            programs.shellcheck.enable = true;
            programs.deadnix.enable = true;
          };
        };
    };
}
