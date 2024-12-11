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
    kernelPackages = pkgs.linuxPackages_zen;
    loader = {
      systemd-boot = {
        enable = false;
        configurationLimit = 5;
      };
      efi = {
        canTouchEfiVariables = true;
        efiSysMountPoint = "/boot";
      };
      grub = {
        enable = true;
        device = "nodev";
        efiSupport = true;
        useOSProber = true;
        configurationLimit = 5;
        catppuccin.enable = true;
      };
    };
  };

  nixosModules.desktop.battery.enable = true;
  nixosModules.desktop.openssh.enable = true;

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

  fonts = {
    fontconfig = {
      antialias = true;
      cache32Bit = true;
      hinting.enable = true;
      hinting.autohint = true;
      subpixel.rgba = "rgb";
    };
    fontDir.enable = true;
    enableDefaultPackages = true;
    packages = with pkgs; [
      corefonts # Microsoft free fonts
      dejavu_fonts
      fira
      fira-mono
      google-fonts
      source-code-pro
      source-sans-pro
      source-serif-pro
      ubuntu_font_family # Ubuntu fonts
      unifont # some international languages
      noto-fonts
      noto-fonts-cjk-sans
      noto-fonts-emoji
      liberation_ttf
      fira-code
      fira-code-symbols
      mplus-outline-fonts.githubRelease
      dina-font
      proggyfonts
      (nerdfonts.override {
        fonts = [
          "CascadiaCode"
          "CascadiaMono"
          "DroidSansMono"
          "Ubuntu"
          "UbuntuMono"
          "UbuntuSans"
        ];
      })
    ];
  };

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
    nvidia.open = false;
    nvidia.nvidiaSettings = true;
  };
  services.xserver.videoDrivers = [ "nvidia" ];

  security.rtkit.enable = true;

  environment = {
    sessionVariables = {
      NIXOS_OZONE_WL = "1";
    };
    systemPackages = with pkgs; [
      age-plugin-yubikey
      headsetcontrol
    ];
  };
  networking.hostName = "${hostname}";
}
