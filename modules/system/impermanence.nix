{ inputs, ... }:
{
  flake.modules.nixos.impermanence =
    { lib, ... }:
    {
      imports = [ inputs.impermanence.nixosModules.impermanence ];

      disko.devices.disk.main.content.partitions.root.content.mountpoint = lib.mkForce "/persist";

      fileSystems."/persist".neededForBoot = true;

      disko.devices.nodev."/" = {
        fsType = "tmpfs";
        mountOptions = [
          "defaults"
          "size=2G"
          "mode=755"
        ];
      };

      environment.persistence."/persist" = {
        hideMounts = true;
        directories = [
          "/nix"
          "/etc/nixos"
          "/srv"
          "/etc/NetworkManager/system-connections"
          "/var/spool"
          "/var/cache/"
          "/var/db/sudo/"
          "/var/lib/nixos"
          "/var/lib/systemd/coredump"
          "/var/lib/systemd/timers"
          "/var/lib/systemd/timesync"
          "/var/lib/bluetooth"
          "/var/lib/NetworkManager"
          "/var/lib/dbus"
          "/var/lib/docker"
          "/var/log"
          "/root"
          "/tmp"
        ];
        files = [
          "/etc/machine-id"
          "/etc/adjtime"
          "/etc/ssh/ssh_host_ed25519_key"
          "/etc/ssh/ssh_host_ed25519_key.pub"
        ];
      };
    };
}
