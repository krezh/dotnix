{
  inputs,
  pkgs,
  lib,
  config,
  hostname,
  ...
}:
{
  imports =
    [
      #inputs.hyprland.nixosModules.default
      inputs.nixos-cosmic.nixosModules.default
    ]
    ++ (lib.scanPath.toList { path = ../common/users; })
    ++ (lib.scanPath.toList { path = ../common/global; });

  boot = {
    plymouth = {
      enable = true;
      catppuccin.enable = true;
    };
    initrd.verbose = false;
    consoleLogLevel = 0;
    kernelParams = [
      "quiet"
      "udev.log_level=0"
    ];
    kernelPackages = pkgs.linuxPackages_latest;
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
        catppuccin.enable = true;
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
    battery.enable = true;
    openssh.enable = true;
    fonts.enable = true;
  };

  security.pam.services.hyprlock = { };

  services.desktopManager.cosmic.enable = false;

  services = {
    displayManager = {
      sddm = {
        enable = true;
        wayland.enable = true;
        autoNumlock = true;
        package = pkgs.kdePackages.sddm;
        catppuccin.enable = true;
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
    #package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    #portalPackage = inputs.xdg-portal-hyprland.packages.${pkgs.system}.default;
  };

  hardware = {
    graphics = {
      enable = true;
    };
    nvidia.package = config.boot.kernelPackages.nvidiaPackages.stable;
    nvidia.modesetting.enable = true;
    nvidia.powerManagement.enable = false;
    nvidia.powerManagement.finegrained = false;
    nvidia.open = true;
    nvidia.nvidiaSettings = true;
  };
  services.xserver.videoDrivers = [ "nvidia" ];

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
      LIBVA_DRIVER_NAME = "nvidia";
      __GLX_VENDOR_LIBRARY_NAME = "nvidia";
    };
    systemPackages = with pkgs; [
      age-plugin-yubikey
      headsetcontrol
      mangohud
      lutris
      protonup
      heroic
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
