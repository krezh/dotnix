{inputs, ...}: {
  flake.modules.nixos.steamdeck = {...}: {
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
      system-desktop
      hyprland
      openssh
      battery
      mount
      krezh
      inputs.jovian.nixosModules.default
    ];

    networking.hostName = "steamdeck";

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

    swapDevices = [
      {
        device = "/var/lib/swapfile";
        size = 16 * 1024;
      }
    ];

    networking.networkmanager.enable = true;

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
  };
}
