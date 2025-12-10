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
    inputs.buildbot-nix.nixosModules.buildbot-worker
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
      buildbot-worker-password = {
        owner = "root";
        mode = "0400";
        path = "/var/lib/secrets/buildbot-worker-password";
      };
      attic-token = {
        owner = "root";
        mode = "0400";
        path = "/var/lib/secrets/attic-token";
      };
      github-token = {
        owner = "root";
        mode = "0400";
        path = "/var/lib/secrets/github-token";
      };
      "attic/netrc-file-pull-push" = {
        sopsFile = ../secrets.yaml;
      };
    };
  };

  # Auto-upgrade from main branch
  system.autoUpgrade = {
    enable = true;
    flake = "github:krezh/nix-config#buildbot-worker";
    dates = "hourly";
    allowReboot = false;
  };

  services.buildbot-nix.worker = {
    enable = true;
    workerPasswordFile = config.sops.secrets.buildbot-worker-password.path;
    # Number of workers (0 = number of CPU cores)
    workers = 0;
    masterUrl = "tcp:host=buildbot-nix-master-pool-0:port=9989";
  };

  systemd.services.buildbot-worker.serviceConfig.Environment = lib.mkForce [
    "WORKER_PASSWORD_FILE=%d/worker-password-file"
    # Use the runtime hostname instead of build-time config
    "WORKER_NAME=%H" # %H is systemd specifier for hostname
  ];

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

  nix = {
    distributedBuilds = true;
    settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      trusted-users = [ "buildbot-worker" ];
      keep-derivations = true;
      fallback = true;
      max-jobs = "auto";
      cores = 0;
      system-features = [
        "big-parallel"
        "kvm"
        "benchmark"
      ];
    };
    extraOptions = ''
      !include ${config.sops.templates."nix_access_token.conf".path}
    '';
    gc = {
      automatic = true;
      dates = "daily";
      options = "--delete-older-than 7d";
    };
  };

  environment.systemPackages = with pkgs; [
    neovim
    gitMinimal
    attic-client
  ];

  nix.settings.netrc-file = config.sops.secrets."attic/netrc-file-pull-push".path;

  # Attic watch-store service to push builds to cache
  systemd.services.attic-watch-store = {
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    environment.HOME = "/var/lib/attic-watch-store";
    serviceConfig = {
      DynamicUser = true;
      MemoryHigh = "5%";
      MemoryMax = "10%";
      LoadCredential = "attic-token:${config.sops.secrets.attic-token.path}";
      StateDirectory = "attic-watch-store";
    };
    path = [ pkgs.attic-client ];
    script = ''
      set -eux -o pipefail
      ATTIC_TOKEN=$(< $CREDENTIALS_DIRECTORY/attic-token)
      attic login krezh http://attic.default.svc.cluster.local:8080/krezh $ATTIC_TOKEN
      exec attic watch-store krezh:krezh
    '';
  };

  time.timeZone = "Europe/Stockholm";
  console.keyMap = "sv-latin1";
  system.stateVersion = "24.05";
}
