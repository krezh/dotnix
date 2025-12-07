{
  pkgs,
  lib,
  modulesPath,
  config,
  ...
}:
{
  nixpkgs.hostPlatform = "x86_64-linux";

  boot.loader.grub.device = "nodev";

  fileSystems."/" = {
    device = "/dev/vda";
    fsType = "ext4";
  };

  system.build.kubevirtImage = lib.mkForce (
    import "${toString modulesPath}/../lib/make-disk-image.nix" {
      inherit lib config pkgs;
      inherit (config.image) baseName;
      format = "qcow2-compressed";
    }
  );

  boot.kernelModules = [ "virtio_balloon" ];

  nix.gc = {
    automatic = true;
    dates = "daily";
    options = "--delete-older-than 7d";
  };

  systemd.services.nix-daemon.serviceConfig = {
    MemoryAccounting = true;
    MemoryMax = "90%";
    OOMScoreAdjust = 500;
  };

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

  security.sudo.wheelNeedsPassword = false;
}
