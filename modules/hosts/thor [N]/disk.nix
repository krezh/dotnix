{inputs, ...}: {
  flake.modules.nixos.thor = {
    imports = [inputs.disko.nixosModules.disko];

    fileSystems."/home".neededForBoot = true;

    disko.devices = {
      disk = {
        main = {
          device = "/dev/nvme0n1";
          type = "disk";
          content = {
            type = "gpt";
            partitions = {
              ESP = {
                type = "EF00";
                size = "1G";
                content = {
                  type = "filesystem";
                  format = "vfat";
                  mountpoint = "/boot";
                };
              };
              root = {
                size = "200G";
                content = {
                  type = "filesystem";
                  format = "ext4";
                  mountpoint = "/";
                };
              };
              swap = {
                size = "16G";
                content = {
                  type = "swap";
                  discardPolicy = "both";
                  resumeDevice = true;
                };
              };
              home = {
                size = "100%";
                content = {
                  type = "filesystem";
                  format = "ext4";
                  mountpoint = "/home";
                };
              };
            };
          };
        };
        secondary = {
          device = "/dev/nvme1n1";
          type = "disk";
          content = {
            type = "gpt";
            partitions = {
              root = {
                size = "100%";
                content = {
                  type = "filesystem";
                  format = "ext4";
                  mountpoint = "/mnt/secondary";
                };
              };
            };
          };
        };
      };
    };
  };
}
