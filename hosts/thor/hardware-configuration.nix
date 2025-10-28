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

  console.earlySetup = false;

  services.xserver.videoDrivers = [ "modesetting" ];
  boot = {
    initrd = {
      verbose = false;
      availableKernelModules = [
        "nvme"
        "ahci"
        "xhci_pci"
        "thunderbolt"
        "usbhid"
      ];
    };
    kernelModules = [ "kvm-amd" ];
    kernel.sysctl = {
      "kernel.nmi_watchdog" = 0;
      "kernel.sched_bore" = "1";
      "vm.swappiness" = 1;
    };
    consoleLogLevel = 0;
    kernelParams = [
      "ipv6.disable=1"
      "split_lock_detect=off"
      "quiet"
      "loglevel=3"
      "systemd.show_status=auto"
      "udev.log_level=3"
      "rd.udev.log_level=3"
      "vt.global_cursor_default=0"
      "module_blacklist=radeon"
      "nvme_core.default_ps_max_latency_us=0"
    ];
  };

  hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}
