{
  config,
  pkgs,
  inputs,
  lib,
  modulesPath,
  ...
}:
{
  imports = [
    inputs.buildbot-nix.nixosModules.buildbot-master
  ];

  # Image configuration for kubevirt
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

  # Enable cloud-init for initial configuration
  services.cloud-init = {
    enable = true;
    #network.enable = true;
  };

  #networking.useDHCP = false;

  # Configure sops-nix for secrets management
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

    # Workers configuration from secrets
    # For KubeVirt pool, create a JSON with workers named buildbot-worker-0, buildbot-worker-1, etc.
    # Format: [{ "name": "buildbot-worker-0", "pass": "password", "cores": 8 }, ...]
    workersFile = config.sops.secrets.buildbot-workers.path;

    # Admin users who can reload projects
    admins = [ "krezh" ];

    # GitHub configuration
    authBackend = "github";
    github = {
      appId = 2429109;
      appSecretKeyFile = config.sops.secrets.github-buildbot-app-secret.path;
      webhookSecretFile = config.sops.secrets.github-buildbot-webhook-secret.path;
      oauthId = "Iv23liKK17hfsMn4RnSK";
      oauthSecretFile = config.sops.secrets.github-buildbot-oauth-secret.path;
      topic = "buildbot-nix";
    };

    # Evaluation settings
    # evalWorkerCount = 4;
    # evalMaxMemorySize = 2048;
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
    settings = {
      substituters = [
        "https://cache.nixos.org"
        "https://nix-cache.plexuz.xyz/krezh"
        "https://nix-community.cachix.org"
        "https://catppuccin.cachix.org"
      ];
      trusted-public-keys = [
        "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
        "krezh:bCYQVVbREhrYgC42zUMf99dMtVXIATXMCcq+wRimqCc="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
        "catppuccin.cachix.org-1:noG/4HkbhJb+lUAdKrph6LaozJvAeEEZj4N732IysmU="
      ];
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
}
