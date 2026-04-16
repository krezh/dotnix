{
  flake.modules.nixos.phone =
    { ... }:
    {
      boot = {
        initrd = {
          availableKernelModules = [ "virtio_pci" ];
          kernelModules = [ ];
        };
        kernelModules = [ ];
        extraModulePackages = [ ];
      };

      fileSystems."/" = {
        device = "/dev/disk/by-uuid/f222513b-ded1-49fa-b591-20cd86a2fe7f";
        fsType = "ext4";
      };

      fileSystems."/boot" = {
        device = "/dev/disk/by-uuid/12CE-A600";
        fsType = "vfat";
        options = [
          "fmask=0022"
          "dmask=0022"
        ];
      };

      fileSystems."/mnt/internal" = {
        device = "internal";
        fsType = "virtiofs";
      };

      fileSystems."/mnt/shared" = {
        device = "android";
        fsType = "virtiofs";
      };

      swapDevices = [ ];
    };
}
