{
  description = "Krezh's NixOS Flake";

  nixConfig = {
    extra-trusted-substituters = [
      "https://krezh.cachix.org"
      "https://cache.garnix.io"
      #"https://nix-cache.plexuz.xyz/krezh"
      "https://nix-community.cachix.org"
      "https://hyprland.cachix.org"
      "https://anyrun.cachix.org"
      "https://walker.cachix.org"
      "https://walker-git.cachix.org"
    ];
    extra-trusted-public-keys = [
      "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
      #"krezh:pqkm/pHp8LD52mFQdGjZR1Xo7RvaG3KdBK4r4FvxIlA="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
      "anyrun.cachix.org-1:pqBobmOjI7nKlsUMV25u9QHa9btJK65/C8vnO3p346s="
      "walker.cachix.org-1:fG8q+uAaMqhsMxWjwvk0IMb4mFPFLqHjuvfwQxE4oJM="
      "walker-git.cachix.org-1:vmC0ocfPWh0S/vRAQGtChuiZBTAe4wiKDeyyXM0/7pM="
      "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    hardware.url = "github:nixos/nixos-hardware";
    lix-module.url = "https://git.lix.systems/lix-project/nixos-module/archive/2.91.0.tar.gz";
    lix-module.inputs.nixpkgs.follows = "nixpkgs";
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";

    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-ld = {
      url = "github:Mic92/nix-ld";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    attic.url = "github:zhaofengli/attic";
    attic.inputs.nixpkgs.follows = "nixpkgs";

    nix-update = {
      url = "github:Mic92/nix-update";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-init = {
      url = "github:nix-community/nix-init";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-inspect.url = "github:bluskript/nix-inspect";
    nix-inspect.inputs.nixpkgs.follows = "nixpkgs";
    catppuccin.url = "github:catppuccin/nix";

    nix-index = {
      url = "github:nix-community/nix-index-database";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixd = {
      url = "github:nix-community/nixd";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nh = {
      url = "github:viperml/nh";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    ### Hyprland
    hyprland.url = "git+https://github.com/hyprwm/Hyprland?submodules=1";
    xdg-portal-hyprland.url = "github:hyprwm/xdg-desktop-portal-hyprland";

    hyprland-plugins = {
      url = "github:hyprwm/hyprland-plugins";
      inputs.hyprland.follows = "hyprland";
    };

    hyprfocus = {
      url = "github:/pyt0xic/hyprfocus";
      inputs.hyprland.follows = "hyprland";
    };

    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hypridle = {
      url = "github:hyprwm/hypridle";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprlock = {
      url = "github:hyprwm/hyprlock";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprkeys = {
      url = "github:hyprland-community/hyprkeys";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    swww = {
      url = "github:LGFae/swww";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    ags = {
      url = "github:Aylur/ags";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprpanel.url = "github:Jas-SinghFSU/HyprPanel";

    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixvim = {
      url = "github:nix-community/nixvim";
      #inputs.nixpkgs.follows = "nixpkgs";
    };

    nixos-wsl = {
      url = "github:nix-community/NixOS-WSL";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    wezterm = {
      url = "github:wez/wezterm?dir=nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    anyrun = {
      url = "github:anyrun-org/anyrun";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    walker.url = "github:abenz1267/walker";
    walker.inputs.nixpkgs.follows = "nixpkgs";

    nixos-grub-themes.url = "github:jeslie0/nixos-grub-themes";

    browser-previews = {
      url = "github:nix-community/browser-previews";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    jovian = {
      url = "github:Jovian-Experiments/Jovian-NixOS";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      flake-parts,
      nixpkgs,
      self,
      ...
    }:
    let
      inherit (self) outputs;

      lib = nixpkgs.lib // import ./lib { inherit lib; };

      nixosSystem =
        hostNames:
        lib.genAttrs hostNames (
          hostName:
          lib.nixosSystem {
            specialArgs = {
              inherit
                self
                inputs
                outputs
                lib
                ;
            };
            modules = lib.scanPath.toList { path = ./hosts/${hostName}; };
          }
        );
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.pre-commit-hooks.flakeModule
        inputs.devshell.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "x86_64-darwin"
      ];

      flake = {
        nixosConfigurations = nixosSystem [
          "thor-wsl"
          "odin"
          "steamdeck"
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
            ghSystem = lib.mapToGha self.nixosConfigurations.${host}.pkgs.system;
          }) (builtins.attrNames self.nixosConfigurations);
        };

        commonModules = lib.scanPath.toList { path = ./modules/common; };

        nixosModules.default = {
          imports = (lib.scanPath.toList { path = ./modules/nixos; });
        };

        overlays = import ./overlays { inherit inputs lib; };
      };

      perSystem =
        { pkgs, config, ... }:
        {
          pre-commit = {
            check.enable = true;
            settings = {
              hooks = {
                nixfmt.enable = true;
                nixfmt.package = pkgs.nixfmt-rfc-style;
                deadnix.enable = true;
                shellcheck.enable = true;
                check-shebang-scripts-are-executable.enable = true;
                check-case-conflicts.enable = true;
                check-json.enable = true;
                lua-ls.enable = true;
                luacheck.enable = true;
              };
            };
          };

          devshells.default = {
            devshell.startup.pre-commit-hook.text = config.pre-commit.installationScript;
            devshell = {
              name = "Default";
              motd = ''
                ❄️ Welcome to the {14}{bold}Default{reset} shell ❄️
              '';
            };
            commands = [
              {
                name = "gpa";
                command = "${pkgs.git}/bin/git pull --autostash && ${pkgs.nh}/bin/nh os switch --ask --no-specialisation";
                help = "git pull and os switch";
                category = "Nix";
              }
            ];
          };

          packages = import ./pkgs { inherit pkgs lib; };
          formatter = pkgs.nixfmt-rfc-style;
        };
    };

}
