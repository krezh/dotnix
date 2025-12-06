{
  pkgs,
  lib,
  modulesPath,
  config,
  ...
}:
{
  nixpkgs.hostPlatform = "x86_64-linux";

  system.build.kubevirtImage = lib.mkForce (
    import "${toString modulesPath}/../lib/make-disk-image.nix" {
      inherit lib config pkgs;
      inherit (config.image) baseName;
      format = "qcow2-compressed";
    }
  );

  users.users.remotebuild = {
    isSystemUser = true;
    group = "remotebuild";
    useDefaultShell = true;

    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBGOPhmqeY3GmqXDK2XHXM6csikAIVzBL+zziXB6LR2F remote-build"
    ];
  };

  users.groups.remotebuild = { };

  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBGOPhmqeY3GmqXDK2XHXM6csikAIVzBL+zziXB6LR2F remote-build"
  ];

  nix = {
    nrBuildUsers = 64;
    settings = {
      trusted-users = [
        "root"
        "remotebuild"
      ];
      min-free = 10 * 1024 * 1024;
      max-free = 200 * 1024 * 1024;
      max-jobs = "auto";
      cores = 0;
      builders-use-substitutes = true;
      use-cgroups = true;
      system-features = [ "big-parallel" ];
      experimental-features = [
        "nix-command"
        "flakes"
        "cgroups"
      ];
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
    };
  };

  systemd.services.nix-daemon.serviceConfig = {
    MemoryAccounting = true;
    MemoryMax = "90%";
    OOMScoreAdjust = 500;
  };

  nix.gc = {
    automatic = true;
    dates = "daily";
    options = "--delete-older-than 7d";
  };

  boot.kernelModules = [ "virtio_balloon" ];

  systemd.services.drop-caches = {
    description = "Drop memory caches after builds";
    serviceConfig = {
      Type = "oneshot";
      User = "root";
      ExecStart = "${pkgs.bash}/bin/bash -c 'sync && echo 3 > /proc/sys/vm/drop_caches'";
    };
  };

  systemd.timers.drop-caches = {
    description = "Periodically drop memory caches";
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnBootSec = "5min";
      OnUnitActiveSec = "30min";
    };
  };

  environment.systemPackages = [
    pkgs.neovim
    pkgs.gitMinimal
  ];

  time.timeZone = "Europe/Stockholm";
  console.keyMap = "sv-latin1";

  system.stateVersion = "24.05";

  security = {
    sudo.wheelNeedsPassword = false;
  };
}
