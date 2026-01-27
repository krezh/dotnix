{
  flake.modules.nixos.thor =
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

      boot = {
        initrd = {
          verbose = false;
          availableKernelModules = [
            "nvme"
            "ahci"
            "xhci_pci"
            "thunderbolt"
            "usbhid"
            "i2c-dev"
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
          "split_lock_detect=off"
          "quiet"
          "loglevel=3"
          "systemd.show_status=auto"
          "udev.log_level=3"
          "rd.udev.log_level=3"
          "vt.global_cursor_default=0"
          "module_blacklist=radeon"
          "nvme_core.default_ps_max_latency_us=0"
          "usbcore.autosuspend=-1"
          # ZSwap
          "zswap.enabled=1" # enables zswap
          "zswap.compressor=lz4" # compression algorithm
          "zswap.max_pool_percent=20" # maximum percentage of RAM that zswap is allowed to use
          "zswap.shrinker_enabled=1" # whether to shrink the pool proactively on high memory pressure
        ];
      };

      hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
    };
}
