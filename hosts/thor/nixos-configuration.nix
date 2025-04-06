{
  pkgs,
  lib,
  hostname,
  ...
}:
{
  imports =
    [ ]
    ++ (lib.scanPath.toList { path = ../common/users; })
    ++ (lib.scanPath.toList { path = ../common/global; });

  boot = {
    plymouth = {
      enable = true;
    };
    initrd.verbose = false;
    consoleLogLevel = 0;
    kernelParams = [
      "quiet"
      "udev.log_level=0"
    ];
    kernelPackages = pkgs.linuxPackages_zen;
    loader = {
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

  programs.steam = {
    enable = true;
    remotePlay.openFirewall = true; # Open ports in the firewall for Steam Remote Play
    dedicatedServer.openFirewall = true; # Open ports in the firewall for Source Dedicated Server
    localNetworkGameTransfers.openFirewall = true; # Open ports in the firewall for Steam Local Network Game Transfers
    gamescopeSession.enable = true;
  };

  programs.gamemode.enable = true;

  nixosModules.desktop = {
    openssh.enable = true;
    fonts.enable = true;
  };

  security.pam.services.hyprlock = { };

  catppuccin.plymouth.enable = true;
  catppuccin.sddm.enable = true;

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
      mouse = {
        accelProfile = "flat";
      };
      touchpad = {
        accelProfile = "flat";
      };
    };

    pipewire = {
      enable = true;
      alsa = {
        enable = true;
        support32Bit = true;
      };
      pulse.enable = true;
      jack.enable = true;
      wireplumber.enable = true;
      audio.enable = true;
    };
  };

  services.udev.packages = [ pkgs.headsetcontrol ];

  networking.networkmanager.enable = true;

  programs.gdk-pixbuf.modulePackages = [ pkgs.librsvg ];

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ ];
    allowedUDPPorts = [ ];
  };

  programs.hyprland = {
    enable = true;
  };

  hardware = {
    graphics = {
      enable = true;
    };
  };

  services.xserver.videoDrivers = [ "amdgpu" ];

  security.rtkit.enable = true;

  programs.nix-ld = {
    enable = true;
    package = pkgs.nix-ld-rs;
  };

  environment = {
    sessionVariables = {
      NIXOS_OZONE_WL = "1";
      STEAM_EXTRA_COMPAT_TOOLS_PATHS = "$HOME/.steam/root/compatibilitytools.d";
      WLR_BACKEND = "vulkan";
    };
    systemPackages = with pkgs; [
      age-plugin-yubikey
      headsetcontrol
      mangohud
      lutris
      protonup
      heroic
      lact
      # Steam
      mangohud
      gamemode
      # WINE
      wine
      winetricks
      protontricks
      vulkan-tools
      # Extra dependencies
      # https://github.com/lutris/docs/
      gnutls
      libgpg-error
      freetype
      sqlite
      libxml2
      xml2
      SDL2
    ];
  };
  networking.hostName = "${hostname}";
}
