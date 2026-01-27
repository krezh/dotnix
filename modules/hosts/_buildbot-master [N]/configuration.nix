{ inputs, ... }:
{
  flake.modules.nixos.buildbot-master =
    {
      config,
      pkgs,
      lib,
      modulesPath,
      ...
    }:
    {
      imports = [
        inputs.buildbot-nix.nixosModules.buildbot-master
        inputs.sops-nix.nixosModules.sops
      ];

      nixpkgs.hostPlatform = "x86_64-linux";

      system.build.kubevirtImage = lib.mkForce (
        import "${toString modulesPath}/../lib/make-disk-image.nix" {
          inherit lib config pkgs;
          inherit (config.image) baseName;
          format = "qcow2-compressed";
        }
      );

      # Boot and filesystem configuration
      boot.loader.grub.device = "/dev/vda";
      fileSystems."/" = {
        device = "/dev/disk/by-label/nixos";
        fsType = "ext4";
        autoResize = true;
      };

      # Cloud-init for initial configuration
      services.cloud-init.enable = true;

      # Sops secrets
      sops = {
        defaultSopsFile = ./secrets.sops.yaml;
        age.keyFile = "/var/lib/sops-nix/key.txt";
        templates = {
          "nix_access_token.conf" = {
            owner = "root";
            content = ''
              access-tokens = github.com=${config.sops.placeholder.github-token}
            '';
          };
        };
        secrets = {
          buildbot-workers = {
            owner = "root";
            mode = "0400";
            path = "/var/lib/secrets/buildbot-workers.json";
          };
          github-buildbot-app-secret = {
            owner = "root";
            mode = "0400";
            path = "/var/lib/secrets/github-app-secret";
          };
          github-buildbot-webhook-secret = {
            owner = "root";
            mode = "0400";
            path = "/var/lib/secrets/github-webhook-secret";
          };
          github-buildbot-oauth-secret = {
            owner = "root";
            mode = "0400";
            path = "/var/lib/secrets/github-oauth-secret";
          };
          github-token = {
            owner = "root";
            mode = "0400";
            path = "/var/lib/secrets/github-token";
          };
        };
      };

      # Auto-upgrade from main branch
      system.autoUpgrade = {
        enable = true;
        flake = "github:krezh/nix-config#buildbot-master";
        dates = "hourly";
        allowReboot = false;
      };

      services.buildbot-nix.master = {
        enable = true;
        domain = "buildbot.plexuz.xyz";
        useHTTPS = true;
        workersFile = config.sops.secrets.buildbot-workers.path;
        admins = [ "krezh" ];
        authBackend = "github";
        github = {
          appId = 2429109;
          appSecretKeyFile = config.sops.secrets.github-buildbot-app-secret.path;
          webhookSecretFile = config.sops.secrets.github-buildbot-webhook-secret.path;
          oauthId = "Iv23liKK17hfsMn4RnSK";
          oauthSecretFile = config.sops.secrets.github-buildbot-oauth-secret.path;
          topic = "buildbot-nix";
        };
      };

      # SSH for administration
      services.openssh = {
        enable = true;
        settings = {
          PermitRootLogin = "prohibit-password";
          PasswordAuthentication = false;
        };
      };

      users.users.root.openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
      ];

      networking.firewall.allowedTCPPorts = [
        80
        443
        9989
      ];

      nix = {
        distributedBuilds = true;
        settings = {
          experimental-features = [
            "nix-command"
            "flakes"
          ];
          trusted-users = [ "buildbot" ];
        };
        extraOptions = ''
          !include ${config.sops.templates."nix_access_token.conf".path}
        '';
      };

      environment.systemPackages = with pkgs; [
        neovim
        gitMinimal
      ];

      time.timeZone = "Europe/Stockholm";
      console.keyMap = "sv-latin1";
      system.stateVersion = "24.05";
    };
}
