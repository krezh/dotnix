{
  inputs,
  ...
}:
{
  flake.modules.nixos.steamdeck =
    { pkgs, ... }:
    {
      home-manager.users.krezh = {
        imports = with inputs.self.modules.homeManager; [
          system-desktop
          hyprland
          desktop-shell
          terminal
          editors
          browsers
          launchers
        ];
      };
      imports = with inputs.self.modules.nixos; [
        # System hierarchy
        system-desktop
        hyprland

        # Services
        openssh
        battery
        mount

        # User
        krezh

        # External modules
        inputs.jovian.nixosModules.default
      ];

      networking.hostName = "steamdeck";

      # Boot configuration
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

      # Networking
      networking.networkmanager.enable = true;

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

      # GameMode for additional gaming optimizations
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

      # Additional Proton versions
      programs.steam.extraCompatPackages = [
        pkgs.proton-ge-bin
      ];
    };
}
