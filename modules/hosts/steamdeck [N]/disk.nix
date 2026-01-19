{ inputs, ... }:
{
  flake.modules.nixos.steamdeck = {
    imports = [ inputs.disko.nixosModules.disko ];

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
                size = "2G";
                content = {
                  type = "filesystem";
                  format = "vfat";
                  mountpoint = "/boot";
                  mountOptions = [ "umask=0077" ];
                };
              };
              root = {
                size = "80G";
                content = {
                  type = "filesystem";
                  format = "ext4";
                  mountpoint = "/";
                  mountOptions = [
                    "defaults"
                    "noatime"
                  ];
                  extraArgs = [
                    "-L"
                    "nixos"
                  ];
                };
              };
              home = {
                size = "100%";
                content = {
                  type = "filesystem";
                  format = "ext4";
                  mountpoint = "/home";
                  mountOptions = [
                    "defaults"
                    "noatime"
                  ];
                  extraArgs = [
                    "-L"
                    "home"
                  ];
                };
              };
            };
          };
        };
      };
    };
  };
}
