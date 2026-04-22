{
  flake.modules.nixos.phone = _: {
    boot = {
      initrd = {
        availableKernelModules = [ "virtio_pci" ];
        kernelModules = [ ];
      };
      kernelModules = [ ];
      extraModulePackages = [ ];
    };

    fileSystems = {
      "/" = {
        device = "/dev/disk/by-uuid/f222513b-ded1-49fa-b591-20cd86a2fe7f";
        fsType = "ext4";
      };

      "/boot" = {
        device = "/dev/disk/by-uuid/12CE-A600";
        fsType = "vfat";
        options = [
          "fmask=0022"
          "dmask=0022"
        ];
      };

      "/mnt/internal" = {
        device = "internal";
        fsType = "virtiofs";
      };

      "/mnt/shared" = {
        device = "android";
        fsType = "virtiofs";
      };

      swapDevices = [ ];
    };
  };
}
