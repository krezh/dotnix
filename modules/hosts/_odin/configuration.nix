{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.odin =
    {
      pkgs,
      lib,
      ...
    }:
    {
      home-manager.users.${user} = {
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
        intel
        desktop-utils
        openssh
        hyprland
        docker
        inputs.self.modules.nixos.${user}
      ];

      nixpkgs.overlays = [
        inputs.nix-cachyos-kernel.overlay
      ];

      networking = {
        hostName = "odin";
        networkmanager = {
          enable = true;
          wifi.backend = "iwd";
        };
        wireless.enable = lib.mkForce false;
        firewall = {
          enable = true;
          allowedTCPPorts = [ ];
          allowedUDPPorts = [ ];
        };
      };

      # Boot configuration
      boot = {
        plymouth.enable = true;
        initrd.verbose = false;
        consoleLogLevel = 0;
        kernelParams = [
          "quiet"
          "udev.log_level=0"
        ];
        kernelPackages = pkgs.cachyosKernels.linuxPackages-cachyos-bore-lto;
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
        udev.packages = [ pkgs.headsetcontrol ];
      };

      programs.gdk-pixbuf.modulePackages = [ pkgs.librsvg ];

      security.rtkit.enable = true;

      environment = {
        sessionVariables = {
          NIXOS_OZONE_WL = "1";
        };
        systemPackages = with pkgs; [
          age-plugin-yubikey
        ];
      };
    };
}
