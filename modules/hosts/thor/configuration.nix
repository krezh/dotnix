{ inputs, ... }:
{
  flake.modules.nixos.thor =
    { pkgs, lib, ... }:
    {
      imports = with inputs.self.modules.nixos; [
        system-desktop
        desktop-utils
        secureboot
        amd
        tpm
        openssh
        gaming
        hyprland
        gnome
        docker
        wooting
        inputs.silentSDDM.nixosModules.default
        inputs.impermanence.nixosModules.impermanence
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
      };

      programs = {
        silentSDDM = {
          enable = true;
          theme = "catppuccin-mocha";
          settings = { };
        };
        seahorse.enable = true;
        nix-ld.enable = true;
        appimage = {
          enable = true;
          binfmt = true;
        };
        sniffnet.enable = true;
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
          };
          efi = {
            canTouchEfiVariables = true;
            efiSysMountPoint = "/boot";
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
      };

      environment = {
        systemPackages = with pkgs; [
          age-plugin-yubikey
          age-plugin-fido2-hmac
          nautilus
          libnotify
          pwvucontrol
          alsa-utils
          pavucontrol
          pulseaudio
        ];

        # Impermanence - persist system state
        persistence."/persist" = {
          hideMounts = true;
          directories = [
            "/nix"
            "/etc/nixos"
            "/srv"
            "/etc/NetworkManager/system-connections"
            "/var/spool"
            "/var/cache/"
            "/var/db/sudo/"
            "/var/lib/nixos"
            "/var/lib/systemd/coredump"
            "/var/lib/systemd/timers"
            "/var/lib/systemd/timesync"
            "/var/lib/bluetooth"
            "/var/lib/NetworkManager"
            "/var/lib/dbus"
            "/var/lib/docker"
            "/var/log"
            "/root"
            "/tmp"
          ];
          files = [
            "/etc/machine-id"
            "/etc/adjtime"
            "/etc/ssh/ssh_host_ed25519_key"
            "/etc/ssh/ssh_host_ed25519_key.pub"
          ];
        };
      };
    };
}
