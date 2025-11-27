{
  pkgs,
  hostname,
  ...
}:
{
  boot = {
    plymouth.enable = true;
    loader = {
      timeout = 0;
      systemd-boot = {
        enable = true;
        configurationLimit = 10;
      };
      efi.canTouchEfiVariables = true;
    };
    resumeDevice = "/dev/disk/by-label/nixos";
  };

  # Swap file for suspend support
  swapDevices = [
    {
      device = "/var/lib/swapfile";
      size = 16 * 1024;
    }
  ];

  # Networking - required for Steam Deck UI setup
  networking = {
    hostName = hostname;
    networkmanager.enable = true;
  };

  # Jovian handles: audio, graphics, steam, kernel params, bluetooth, SD card automount
  jovian = {
    devices.steamdeck.enable = true;
    steam = {
      enable = true;
      autoStart = true;
      user = "krezh";
      desktopSession = "hyprland";
    };
    decky-loader.enable = true;
  };

  # Desktop environment for "Switch to Desktop"
  programs.hyprland.enable = true;

  # Optional: GameMode for additional gaming optimizations
  programs.gamemode = {
    enable = true;
    settings = {
      general.renice = 10;
      gpu = {
        apply_gpu_optimisations = "accept-responsibility";
        gpu_device = 0;
        amd_performance_level = "high";
      };
    };
  };

  # Optional: Additional Proton versions
  programs.steam.extraCompatPackages = [
    pkgs.proton-ge-bin
  ];
}
