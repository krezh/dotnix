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
          # Modules the initrd needs to reach the root filesystem and the keyboard
          availableKernelModules = [
            "nvme" # root disk
            "ahci"
            "xhci_pci" # USB 3 controller
            "thunderbolt"
            "usbhid" # USB keyboard, for the disk unlock prompt
            "i2c-dev"
          ];
        };
        kernelModules = [ "kvm-amd" ]; # AMD hardware virtualization
        kernel.sysctl = {
          "kernel.nmi_watchdog" = 0; # off: costs a little power/latency, only useful for debugging hard lockups
          "kernel.sched_bore" = "1"; # CachyOS BORE scheduler; favours interactive tasks
          "vm.swappiness" = 1; # swap only under real memory pressure — zswap absorbs the rest
        };
        consoleLogLevel = 0;
        kernelParams = [
          "split_lock_detect=off" # don't stall the offending task on split-lock atomics (games, emulators)
          # Quiet boot
          "quiet"
          "loglevel=3" # kernel messages: errors and above
          "systemd.show_status=auto"
          "udev.log_level=3"
          "rd.udev.log_level=3" # same, but in the initrd
          "vt.global_cursor_default=0" # no blinking cursor on the console
          "module_blacklist=radeon" # keep the legacy driver off the GPU; amdgpu drives it
          "nvme_core.default_ps_max_latency_us=0" # disable NVMe APST; its power states add wakeup latency
          "usbcore.autosuspend=-1" # never autosuspend USB devices
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
