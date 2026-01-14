{
  inputs,
  ...
}:
{
  flake.modules.nixos.thor =
    {
      pkgs,
      lib,
      ...
    }:
    {
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

      nixpkgs.overlays = [
        inputs.nix-cachyos-kernel.overlay
      ];

      networking.hostName = "thor";

      # Display manager
      services.displayManager = {
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

      # Boot configuration
      boot = {
        plymouth.enable = true;
        kernelPackages = pkgs.cachyosKernels.linuxPackages-cachyos-bore-lto;
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
      };

      # System services
      services.fwupd.enable = true;
      services.accounts-daemon.enable = true;
      services.gnome.gnome-online-accounts.enable = true;
      services.scx.enable = false;
      services.scx.scheduler = "scx_bpfland";

      # Disable coredump
      systemd.coredump.enable = false;
      boot.kernel.sysctl = {
        "kernel.core_pattern" = "|/bin/false";
        "kernel.core_uses_pid" = 0;
      };

      security.pam.loginLimits = [
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
      security.pam.services.krezh.enableGnomeKeyring = true;
      security.pam.services.sddm.enableGnomeKeyring = true;
      security.pam.services.hyprlock.enableGnomeKeyring = true;
      security.pam.services.login.enableGnomeKeyring = true;
      programs.seahorse.enable = true;
      services.gnome.gnome-keyring.enable = true;
      services.dbus.packages = with pkgs; [
        gnome-keyring
        gcr
        seahorse
        libsecret
        libgnome-keyring
      ];

      # OpenRGB
      services.hardware.openrgb = {
        enable = false;
        package = pkgs.openrgb-with-all-plugins;
        motherboard = "amd";
      };

      # Misc services
      services.fstrim.enable = true;
      services.libinput = {
        enable = true;
        mouse.accelProfile = "flat";
        touchpad.accelProfile = "flat";
      };
      security.rtkit.enable = true;

      # Programs
      programs.nix-ld.enable = true;
      programs.appimage = {
        enable = true;
        binfmt = true;
      };
      programs.evolution.enable = true;
      programs.sniffnet.enable = true;

      # Networking
      services.timesyncd.servers = [ ];
      networking.networkmanager.enable = true;
      networking.networkmanager.wifi.backend = "iwd";
      networking.wireless.enable = lib.mkForce false;

      networking.firewall =
        let
          kde-connect = [
            {
              from = 1714;
              to = 1764;
            }
          ];
        in
        {
          allowedTCPPortRanges = kde-connect;
          allowedUDPPortRanges = kde-connect;
        };

      # AMD GPU
      hardware = {
        graphics = {
          enable = true;
          enable32Bit = true;
        };
        amdgpu = {
          opencl.enable = true;
          initrd.enable = true;
          overdrive.enable = true;
        };
      };

      environment = {
        sessionVariables = {
          NIXOS_OZONE_WL = 1;
          WLR_BACKEND = "vulkan";
          PROTON_FSR4_UPGRADE = 1;
          AMD_VULKAN_ICD = "RADV";
          MESA_SHADER_CACHE_MAX_SIZE = "50G";
          __GL_SHADER_DISK_CACHE_SKIP_CLEANUP = 1;
        };
        systemPackages = with pkgs; [
          amdgpu_top
          age-plugin-yubikey
          wootility
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

      # Wooting keyboard udev rules
      services.udev.extraRules = ''
        # Wooting One Legacy
        SUBSYSTEM=="hidraw", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="ff01", TAG+="uaccess"
        SUBSYSTEM=="usb", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="ff01", TAG+="uaccess"

        # Wooting One update mode
        SUBSYSTEM=="hidraw", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="2402", TAG+="uaccess"

        # Wooting Two Legacy
        SUBSYSTEM=="hidraw", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="ff02", TAG+="uaccess"
        SUBSYSTEM=="usb", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="ff02", TAG+="uaccess"

        # Wooting Two update mode
        SUBSYSTEM=="hidraw", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="2403", TAG+="uaccess"

        # Generic Wooting devices
        SUBSYSTEM=="hidraw", ATTRS{idVendor}=="31e3", TAG+="uaccess"
        SUBSYSTEM=="usb", ATTRS{idVendor}=="31e3", TAG+="uaccess"
      '';
    };
}
