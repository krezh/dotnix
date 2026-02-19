{ inputs, ... }:
{
  flake.modules.nixos.thor =
    { pkgs, lib, ... }:
    {
      imports = with inputs.self.modules.nixos; [
        system-desktop
        desktop-utils
        amd
        openssh
        gaming
        hyprland
        docker
        wooting
        inputs.silentSDDM.nixosModules.default
      ];

      nixpkgs.overlays = [
        inputs.nix-cachyos-kernel.overlay
      ];

      networking = {
        hostName = "thor";
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

      programs = {
        silentSDDM = {
          enable = true;
          theme = "catppuccin-mocha";
          # settings = { ... }; see example in module
        };
        seahorse.enable = true;
        nix-ld.enable = true;
        appimage = {
          enable = true;
          binfmt = true;
        };
      };

      catppuccin.sddm.enable = false;

      # Display manager
      services = {
        displayManager = {
          sddm = {
            enable = true;
            wayland.enable = true;
            wayland.compositor = "weston";
            autoNumlock = true;
          };
          gdm = {
            enable = false;
            wayland = true;
          };
          defaultSession = "hyprland";
        };

        # System services
        fwupd.enable = true;
        accounts-daemon.enable = true;
        gnome = {
          gnome-online-accounts.enable = true;
          gnome-keyring.enable = true;
        };
        scx = {
          enable = false;
          scheduler = "scx_bpfland";
        };
        dbus.packages = with pkgs; [
          gnome-keyring
          gcr
          seahorse
          libsecret
          libgnome-keyring
        ];

        # Misc services
        fstrim.enable = true;
        libinput = {
          enable = true;
          mouse.accelProfile = "flat";
          touchpad.accelProfile = "flat";
        };
        timesyncd.servers = [ ];

        earlyoom = {
          enable = true;
          freeMemThreshold = 5;
          enableNotifications = true;
        };
      };

      # Boot configuration
      boot = {
        plymouth.enable = true;
        kernelPackages = pkgs.cachyosKernels.linuxPackages-cachyos-bore-lto;
        tmp.cleanOnBoot = true;
        loader = {
          timeout = 0;
          systemd-boot = {
            enable = true;
            configurationLimit = 5;
            netbootxyz.enable = true;
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
        kernel.sysctl = {
          "kernel.core_pattern" = "|/bin/false";
          "kernel.core_uses_pid" = 0;
        };
      };

      # Disable coredump
      systemd = {
        coredump.enable = false;
        oomd.enableUserSlices = true;
      };

      security = {
        pam = {
          loginLimits = [
            {
              domain = "*";
              type = "hard";
              item = "core";
              value = "0";
            }
            {
              domain = "*";
              type = "soft";
              item = "core";
              value = "0";
            }
          ];
          # GNOME keyring
          services = {
            sddm.enableGnomeKeyring = true;
            hyprlock.enableGnomeKeyring = true;
            login.enableGnomeKeyring = true;
          };
        };
        rtkit.enable = true;
      };

      environment = {
        sessionVariables = {
          NIXOS_OZONE_WL = 1;
        };
        systemPackages = with pkgs; [
          age-plugin-yubikey
          nautilus
          libnotify
          pwvucontrol
          alsa-utils
          pavucontrol
          pulseaudio
        ];
      };

      # Wireplumber configuration
      nixosModules.wireplumber = {
        enable = true;
        audioSwitching = {
          enable = true;
          primary = "A50";
          secondary = "Argon Speakers";
        };
        hideNodes = [
          "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__Headphones__sink"
          "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__Speaker__sink"
          "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Mic2__source"
          "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Mic1__source"
          "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Line1__source"
        ];
        renameModules = [
          {
            nodeName = "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__SPDIF__sink";
            description = "Argon Speakers";
            nick = "Argon Speakers";
          }
          {
            nodeName = "alsa_output.usb-Logitech_A50-00.iec958-stereo";
            description = "A50";
            nick = "A50";
          }
          {
            nodeName = "alsa_input.usb-Logitech_A50-00.mono-fallback";
            description = "A50";
            nick = "A50";
          }
        ];
        deviceSettings = {
          "usb-Generic_USB_Audio-00" = {
            priority = 50;
            deviceProps = {
              "device.profile" = "HiFi 5+1";
            };
          };
          "usb-Logitech_A50-00" = {
            priority = 51;
            deviceProps = {
              "api.acp.auto-profile" = "false";
              "device.profile" = "iec958-stereo";
            };
          };
        };
      };
    };
}
