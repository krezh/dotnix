{
  inputs,
  lib,
  ...
}:
let
  # Compute the packages overlay at flake-parts level where we have access to custom lib
  pkgsOverlay =
    final: _prev:
    lib.scanPath.toAttrs {
      path = lib.relativeToRoot "pkgs";
      func = final.callPackage;
      useBaseName = true;
      excludeFiles = [ "vscode-extensions" ];
    };

  # VSCode extensions overlay
  vscodeExtensionsOverlay = final: prev: {
    vscode-extensions = prev.vscode-extensions // {
      theqtcompany = {
        qt-core = final.callPackage (lib.relativeToRoot "pkgs/vscode-extensions/theqtcompany/qt-core") { };
        qt-qml = final.callPackage (lib.relativeToRoot "pkgs/vscode-extensions/theqtcompany/qt-qml") { };
        qt-ui = final.callPackage (lib.relativeToRoot "pkgs/vscode-extensions/theqtcompany/qt-ui") { };
      };
      opentofu = {
        opentofu = final.callPackage (lib.relativeToRoot "pkgs/vscode-extensions/opentofu/opentofu") { };
      };
    };
  };
in
{
  flake.modules.nixos.system-base =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    {
      imports = [
        inputs.sops-nix.nixosModules.sops
        inputs.home-manager.nixosModules.home-manager
      ]
      ++ (with inputs.self.modules; [
        generic.var
        nixos.shell
        nixos.catppuccin
      ]);

      # Nixpkgs configuration
      nixpkgs = {
        config.allowUnfree = true;
        overlays = [
          inputs.gomod2nix.overlays.default
          pkgsOverlay
          vscodeExtensionsOverlay
        ];
      };

      # Home-manager configuration
      home-manager = {
        useGlobalPkgs = true;
        useUserPackages = true;
        backupFileExtension = "bk";
        extraSpecialArgs = {
          inherit inputs;
        };
        sharedModules = [ inputs.sops-nix.homeManagerModules.sops ];
      };

      system.stateVersion = lib.mkDefault "24.05";

      # Locale settings
      i18n = {
        defaultLocale = lib.mkDefault "en_SE.UTF-8";
        # supportedLocales = lib.mkDefault [ "en_US.UTF-8" ];
        extraLocales = "all";
        extraLocaleSettings.LC_TIME = "en_SE.UTF-8";
      };
      console.keyMap = "sv-latin1";
      time.timeZone = "Europe/Stockholm";

      # Nix settings
      nix = {
        package = pkgs.lixPackageSets.stable.lix;
        extraOptions = ''
          !include ${config.sops.templates."nix_access_token.conf".path}
        '';
        settings = {
          keep-outputs = true;
          keep-derivations = true;
          warn-dirty = false;
          flake-registry = "";
          use-xdg-base-directories = true;
          accept-flake-config = true;
          always-allow-substitutes = true;
          builders-use-substitutes = true;
          trusted-users = [
            "@wheel"
            "root"
          ];
          auto-optimise-store = true;
          experimental-features = [
            "nix-command"
            "flakes"
          ];
          system-features = [ ];
          extra-substituters = [
            "https://krezh.cachix.org"
            "https://cache.garnix.io"
            "https://nix-community.cachix.org"
            "https://niri.cachix.org"
          ];
          extra-trusted-public-keys = [
            "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
            "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
            "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
            "niri.cachix.org-1:Wv0OmO7PsuocRKzfDoJ3mulSl7Z6oezYhGhR+3W2964="
          ];
        };
        gc = {
          automatic = true;
          dates = lib.mkDefault "weekly";
        };
        channel.enable = lib.mkForce false;
        registry = lib.mapAttrs (_: value: { flake = value; }) (
          lib.filterAttrs (name: _: name != "self") inputs
        );
        nixPath = lib.mkForce [ "nixpkgs=${inputs.nixpkgs}" ];
      };

      # Environment variables
      environment.variables = {
        TZ = "Europe/Stockholm";
      };

      # Hardware
      hardware.enableRedistributableFirmware = true;

      # Services
      services.pcscd.enable = true;

      # Security
      security = {
        sudo = {
          enable = true;
          wheelNeedsPassword = lib.mkDefault true;
          extraRules = [
            {
              commands = [
                {
                  command = "${pkgs.systemd}/bin/reboot";
                  options = [ "NOPASSWD" ];
                }
              ];
              groups = [ "wheel" ];
            }
            {
              commands = [
                {
                  command = "/run/current-system/sw/bin/true";
                  options = [ "NOPASSWD" ];
                }
              ];
              users = [ "root" ];
            }
          ];
          extraConfig = ''
            Defaults pwfeedback
            Defaults timestamp_timeout=15
          '';
        };
        pam.loginLimits = [
          {
            domain = "@wheel";
            item = "nofile";
            type = "soft";
            value = "524288";
          }
          {
            domain = "@wheel";
            item = "nofile";
            type = "hard";
            value = "1048576";
          }
        ];
      };

      # Groups
      users.groups.sshusers = { };

      # Base packages
      environment.systemPackages = with pkgs; [
        git
        wget
        deadnix
        nix-init
        nix-update
        nixd
        nil
        nix-inspect
        cachix
        nixfmt
        dix
        nix-output-monitor
        comma
        nix-tree
        nixos-anywhere
        attic-client
        nixos-update
      ];

      programs.nh.enable = true;
      documentation.man.generateCaches = lib.mkForce false;
    };
}
