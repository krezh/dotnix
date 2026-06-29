{ inputs, ... }:
{
  flake.modules.nixos.system-common =
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
        nixos.system-base
        generic.var
        nixos.shell
        nixos.catppuccin
        nixos.modules
      ]);

      home-manager = {
        useGlobalPkgs = true;
        useUserPackages = true;
        backupFileExtension = "bk";
        extraSpecialArgs = { inherit inputs; };
        sharedModules = [ inputs.sops-nix.homeManagerModules.sops ];
      };

      console.enable = false;
      services.kmscon = {
        enable = true;
        config = {
          hwaccel = false;
          font-name = "${config.var.fonts.mono} Bold";
          font-size = 20;
          xkb-layout = "se";
          xkb-variant = "nodeadkeys";
        };
      };

      networking.nftables.enable = true;

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
        nix-fast-build
        nix-eval-jobs
        inputs.go-overlay.packages.${pkgs.stdenv.hostPlatform.system}.govendor
      ];

      programs.nh = {
        enable = true;
      };
    };
}
