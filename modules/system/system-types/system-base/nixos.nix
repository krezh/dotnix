{ inputs, ... }:
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
        nixos.modules
      ]);

      nixpkgs = {
        config.allowUnfree = true;
        overlays = [ inputs.self.overlays.default ];
      };

      # Home-manager configuration
      home-manager = {
        useGlobalPkgs = true;
        useUserPackages = true;
        backupFileExtension = "bk";
        extraSpecialArgs = { inherit inputs; };
        sharedModules = [ inputs.sops-nix.homeManagerModules.sops ];
      };

      system.stateVersion = lib.mkDefault "24.05";

      # Locale settings
      i18n = {
        defaultLocale = lib.mkDefault "en_US.UTF-8";
        extraLocales = "all";
        extraLocaleSettings.LC_TIME = "en_US.UTF-8";
      };
      console = {
        enable = false;
        keyMap = "sv-latin1";
      };
      services.kmscon = {
        enable = true;
        hwRender = false;
        extraConfig = ''
          font-name=${config.var.fonts.mono} Bold
          font-size=12
          xkb-layout=se
          xkb-variant=nodeadkeys
        '';
      };

      time.timeZone = "Europe/Stockholm";

      # Nix settings
      nix = {
        package = pkgs.lixPackageSets.latest.lix;
        extraOptions = lib.optionalString (config.sops.templates ? "nix_access_token.conf") ''
          !include ${config.sops.templates."nix_access_token.conf".path}
        '';
        settings = {
          keep-outputs = lib.mkDefault false;
          keep-derivations = lib.mkDefault false;
          warn-dirty = true;
          flake-registry = "";
          use-xdg-base-directories = true;
          accept-flake-config = true;
          always-allow-substitutes = true;
          builders-use-substitutes = true;
          auto-optimise-store = true;
          trusted-users = [
            "@wheel"
          ];
          experimental-features = [
            "nix-command"
            "flakes"
            "cgroups"
          ];
          system-features = [ ];
          extra-substituters = [
            "https://nix-cache.plexuz.xyz/krezh"
            "https://nix-community.cachix.org"
            "https://niri.cachix.org"
          ];
          extra-trusted-public-keys = [
            "krezh:GBrZyWDPWYTg/9a9Vad/NRQF/1w0Yc1kWXOQXM3d0RQ="
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
        sudo-rs = {
          enable = true;
          wheelNeedsPassword = lib.mkDefault true;
          extraConfig = ''
            Defaults pwfeedback
            Defaults timestamp_timeout=15
          '';
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
          ];
        };
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
        nix-fast-build
        nix-eval-jobs
        inputs.go-overlay.packages.${pkgs.stdenv.hostPlatform.system}.govendor
      ];

      programs.nh = {
        enable = true;
      };
      documentation.man.cache.enable = lib.mkForce false;
      documentation.enable = false;
    };
}
