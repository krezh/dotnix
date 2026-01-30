{ inputs, ... }:
{
  flake.modules.nixos.jotunheim = {
    imports = [ inputs.disko.nixosModules.disko ];

    disko.devices = {
      disk = {
        ssd1 = {
          type = "disk";
          device = "/dev/disk/by-id/ata-KINGSTON_SA400S37120G_50026B77834EF632";
          content = {
            type = "gpt";
            partitions = {
              grub = {
                size = "10M";
                type = "EF02";
              };
              ESP = {
                type = "EF00";
                size = "2G";
                content = {
                  type = "filesystem";
                  format = "vfat";
                  mountpoint = "/boot/efi";
                };
              };
              zfs = {
                size = "100%";
                content = {
                  type = "zfs";
                  pool = "rpool";
                };
              };
            };
          };
        };
        ssd2 = {
          type = "disk";
          device = "/dev/disk/by-id/ata-KINGSTON_SA400S37120G_50026B77834EE7DD";
          content = {
            type = "gpt";
            partitions = {
              grub = {
                size = "10M";
                type = "EF02";
              };
              ESP = {
                type = "EF00";
                size = "2G";
                content = {
                  type = "filesystem";
                  format = "vfat";
                  mountpoint = "/boot/efi-fallback";
                };
              };
              zfs = {
                size = "100%";
                content = {
                  type = "zfs";
                  pool = "rpool";
                };
              };
            };
          };
        };
      };

      zpool = {
        rpool = {
          type = "zpool";
          mode = "mirror";
          options = {
            autotrim = "on";
          };
          rootFsOptions = {
            compression = "on";
            atime = "off";
            relatime = "on";
            xattr = "on";
            acltype = "nfsv4";
            recordsize = "128K";
            sync = "standard";
            dedup = "off";
            logbias = "latency";
            canmount = "off";
            mountpoint = "none";
          };

          datasets = {
            root = {
              type = "zfs_fs";
              mountpoint = "/";
              options = {
                mountpoint = "legacy";
                canmount = "on";
              };
            };
            boot = {
              type = "zfs_fs";
              mountpoint = "/boot";
              options = {
                mountpoint = "legacy";
                canmount = "on";
              };
            };
            nix = {
              type = "zfs_fs";
              mountpoint = "/nix";
              options = {
                mountpoint = "legacy";
                canmount = "on";
                atime = "off";
              };
            };
            home = {
              type = "zfs_fs";
              mountpoint = "/home";
              options = {
                mountpoint = "legacy";
                canmount = "on";
              };
            };
            var = {
              type = "zfs_fs";
              mountpoint = "/var";
              options = {
                mountpoint = "legacy";
                canmount = "on";
              };
            };
          };
        };
      };
    };
  };
}
