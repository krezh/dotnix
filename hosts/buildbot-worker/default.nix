{
  config,
  pkgs,
  lib,
  inputs,
  ...
}:
{
  imports = [
    inputs.buildbot-nix.nixosModules.buildbot-worker
    ../../images/buildbot-worker.nix
  ];

  # Configure sops-nix for secrets management
  sops = {
    defaultSopsFile = ./secrets.sops.yaml;
    age.keyFile = "/var/lib/sops-nix/key.txt";

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
    };
  };

  # Auto-upgrade from main branch
  system.autoUpgrade = {
    enable = true;
    flake = "github:krezh/nix-config";
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

  # Override the systemd service to use the runtime hostname as worker name
  systemd.services.buildbot-worker = {
    serviceConfig = {
      Environment = lib.mkForce [
        "WORKER_NAME=%H" # %H expands to the actual hostname at runtime
      ];
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

  nix = {
    settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      trusted-users = [ "buildbot-worker" ];
      max-jobs = "auto";
      cores = 0;
      system-features = [
        "big-parallel"
        "kvm"
        "benchmark"
      ];
      substituters = [
        "https://cache.nixos.org"
        "https://nix-cache.plexuz.xyz/krezh"
        "https://nix-community.cachix.org"
      ];
      trusted-public-keys = [
        "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
        "krezh:bCYQVVbREhrYgC42zUMf99dMtVXIATXMCcq+wRimqCc="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      ];
    };
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
      attic login krezh https://nix-cache.plexuz.xyz/krezh $ATTIC_TOKEN
      attic use krezh
      exec attic watch-store krezh:krezh
    '';
  };

  time.timeZone = "Europe/Stockholm";
  console.keyMap = "sv-latin1";
  system.stateVersion = "24.05";
}
