{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.steamdeck =
    { pkgs, ... }:
    {
      home-manager.users.${user} = {
        imports = with inputs.self.modules.homeManager; [
          system-desktop
          gnome
          desktop-shell
          terminal
          editors
          browsers
          launchers
        ];
      };
      imports = with inputs.self.modules.nixos; [
        system-desktop
        gnome
        openssh
        battery
        inputs.self.modules.nixos.${user}
        inputs.jovian.nixosModules.default
      ];

      services = {
        displayManager.sddm.wayland.enable = true;
      };

      boot = {
        plymouth.enable = true;
        loader = {
          timeout = 0;
          systemd-boot = {
            enable = true;
            configurationLimit = 5;
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
      environment.systemPackages = [ pkgs.wvkbd ];

      jovian = {
        devices.steamdeck.enable = true;
        steam = {
          enable = true;
          autoStart = true;
          inherit user;
          desktopSession = "gnome";
        };
        decky-loader.enable = true;
      };
    };
}
