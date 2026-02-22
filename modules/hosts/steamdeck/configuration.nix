{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.steamdeck =

    {
      home-manager.users.${user} = {
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
        inputs.self.modules.nixos.${user}
        inputs.jovian.nixosModules.default
      ];

      networking.hostName = "steamdeck";

      services.orca.enable = false;

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
          inherit user;
          desktopSession = "hyprland";
        };
        decky-loader.enable = true;
      };
    };
}
