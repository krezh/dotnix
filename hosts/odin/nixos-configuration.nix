{
  inputs,
  pkgs,
  lib,
  config,
  ...
}:
{
  imports =
    [
      inputs.hyprland.nixosModules.default
      inputs.hardware.nixosModules.common-cpu-intel
      # inputs.stylix.nixosModules.stylix

    ]
    ++ (lib.listNixFiles { path = ../common/users; })
    ++ (lib.listNixFiles { path = ../common/global; });

  # stylix.enable = false;
  # stylix.autoEnable = false;
  # stylix.base16Scheme = "${pkgs.base16-schemes}/share/themes/catppuccin-mocha.yaml";
  # stylix.image = config.lib.stylix.pixel "base0A";

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
    kernelPackages = pkgs.linuxPackages_latest; # Use latest kernel
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

  services.displayManager = {
    sddm = {
      enable = true;
      wayland.enable = true;
      autoNumlock = true;
      package = pkgs.kdePackages.sddm;
      catppuccin.enable = true;
    };
    defaultSession = "hyprland";
  };

  networking.networkmanager.enable = true;

  programs.gdk-pixbuf.modulePackages = [ pkgs.librsvg ];

  fonts.packages = with pkgs; [
    (nerdfonts.override {
      fonts = [
        "CascadiaCode"
        "DroidSansMono"
      ];
    })
  ];

  programs.hyprland = {
    enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
  };

  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa = {
      enable = true;
      support32Bit = true;
    };
    pulse = {
      enable = true;
    };
    jack = {
      enable = true;
    };
    wireplumber = {
      enable = true;
    };
    audio = {
      enable = true;
    };
  };

  services.upower.enable = true;

  services.fstrim.enable = true;
  services.libinput = {
    enable = true;
    mouse = {
      accelProfile = "flat";
    };
    touchpad = {
      accelProfile = "flat";
    };
  };

  services.tlp = {
    enable = false;
    settings = {
      PLATFORM_PROFILE_ON_AC = "performance";
      PLATFORM_PROFILE_ON_BAT = "low-power";
    };
  };

  environment = {
    sessionVariables.NIXOS_OZONE_WL = "1";
    systemPackages = with pkgs; [
      liberation_ttf
      noto-fonts-emoji
      age-plugin-yubikey
    ];
  };
  networking.hostName = "odin";
}
