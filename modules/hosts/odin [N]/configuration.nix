{
  inputs,
  ...
}:
{
  flake.modules.nixos.odin =
    {
      pkgs,
      lib,
      ...
    }:
    {
      home-manager.users.krezh = {
        imports = with inputs.self.modules.homeManager; [
          system-desktop
          terminal
          editors
          browsers
          media
          launchers
          mail
          ai
          hyprland
          desktop-shell
          desktop-utils
        ];
      };
      imports = with inputs.self.modules.nixos; [
        system-desktop
        desktop-utils
        openssh
        steam
        wireplumber
        hyprland
        niri
        ai
        docker
        krezh
      ];

      networking.hostName = "odin";

      # Boot configuration
      boot = {
        plymouth.enable = true;
        initrd.verbose = false;
        consoleLogLevel = 0;
        kernelParams = [
          "quiet"
          "udev.log_level=0"
        ];
        kernelPackages = pkgs.linuxPackages_zen;
        loader = {
          timeout = 0;
          systemd-boot = {
            enable = true;
            configurationLimit = 5;
          };
          efi = {
            canTouchEfiVariables = true;
            efiSysMountPoint = "/boot";
          };
          grub = {
            enable = false;
            device = "nodev";
            efiSupport = true;
            useOSProber = true;
            configurationLimit = 5;
          };
        };
      };

      zramSwap.enable = true;

      # Steam
      programs.steam = {
        enable = true;
        remotePlay.openFirewall = true;
        dedicatedServer.openFirewall = true;
        localNetworkGameTransfers.openFirewall = true;
        gamescopeSession.enable = true;
      };

      security.pam.services.hyprlock = { };

      # Display manager
      services = {
        displayManager = {
          sddm = {
            enable = true;
            wayland.enable = true;
            autoNumlock = true;
            package = pkgs.kdePackages.sddm;
          };
          defaultSession = "hyprland";
        };
        gnome.gnome-keyring.enable = true;

        fstrim.enable = true;

        libinput = {
          enable = true;
          mouse.accelProfile = "flat";
          touchpad.accelProfile = "flat";
        };
      };

      services.udev.packages = [ pkgs.headsetcontrol ];

      # Networking
      networking.networkmanager.enable = true;
      networking.networkmanager.wifi.backend = "iwd";
      networking.wireless.enable = lib.mkForce false;

      programs.gdk-pixbuf.modulePackages = [ pkgs.librsvg ];

      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ ];
        allowedUDPPorts = [ ];
      };

      # Intel GPU
      hardware = {
        graphics = {
          enable = true;
          extraPackages = with pkgs; [
            intel-media-driver
            libvdpau-va-gl
          ];
        };
      };

      security.rtkit.enable = true;

      environment = {
        sessionVariables = {
          NIXOS_OZONE_WL = "1";
          LIBVA_DRIVER_NAME = "iHD";
        };
        systemPackages = with pkgs; [
          age-plugin-yubikey
          intel-gpu-tools
        ];
      };
    };
}
