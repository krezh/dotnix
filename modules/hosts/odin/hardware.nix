{ inputs, ... }:
{
  flake.modules.nixos.odin =
    {
      config,
      lib,
      modulesPath,
      ...
    }:
    {
      imports = [
        (modulesPath + "/installer/scan/not-detected.nix")
        inputs.hardware.nixosModules.common-cpu-intel
        inputs.hardware.nixosModules.common-pc-ssd
      ];

      boot = {
        initrd = {
          availableKernelModules = [
            "xhci_pci"
            "thunderbolt"
            "nvme"
            "usb_storage"
            "usbhid"
            "sd_mod"
            "rtsx_usb_sdmmc"
          ];
          kernelModules = [ ];
        };
        kernelParams = [ "ipv6.disable=1" ];
        kernelModules = [ "kvm-intel" ];
        extraModulePackages = [ ];
      };

      swapDevices = [ ];

      hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
    };
}
