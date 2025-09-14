{
  config,
  lib,
  modulesPath,
  ...
}:

{
  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
  ];

  console = {
    earlySetup = false;
  };

  services.xserver.videoDrivers = [ "modesetting" ];
  boot = {
    initrd = {
      verbose = false;
      availableKernelModules = [
        "vmd"
        "xhci_pci"
        "ahci"
        "nvme"
        "usbhid"
        "usb_storage"
        "sd_mod"
      ];
    };
    consoleLogLevel = 0;
    kernelParams = [
      "ipv6.disable=1"
      "intel_pstate=disable"
      "split_lock_detect=off"
      "quiet"
      "loglevel=3"
      "systemd.show_status=auto"
      "udev.log_level=3"
      "rd.udev.log_level=3"
      "vt.global_cursor_default=0"
      "module_blacklist=radeon"
    ];
  };

  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}
