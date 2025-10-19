{
  pkgs,
  inputs,
  lib,
  hostname,
  ...
}:
{
  imports = [
    inputs.chaotic.nixosModules.nyx-cache
    inputs.chaotic.nixosModules.nyx-overlay
    inputs.chaotic.nixosModules.nyx-registry
    ../common/users
    ../common/global
  ];

  boot = {
    plymouth = {
      enable = false;
    };
    kernelPackages = pkgs.linuxPackages_cachyos;
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

  services.fwupd.enable = true;

  services.scx.enable = true;
  services.scx.scheduler = "scx_lavd";

  xdg.mime.enable = true;

  security.pam.services.krezh.enableGnomeKeyring = true;
  security.pam.services.gdm-password.enableGnomeKeyring = true;
  security.pam.services.gdm.enableGnomeKeyring = true;
  security.pam.services.sddm.enableGnomeKeyring = true;
  security.pam.services.hyprlock.enableGnomeKeyring = true;
  security.pam.services.login.enableGnomeKeyring = true;
  security.rtkit.enable = true;
  programs.seahorse.enable = true;
  services.gnome.gnome-keyring.enable = true;
  services.dbus.packages = with pkgs; [
    gnome-keyring
    gcr
    seahorse
    libsecret
    libgnome-keyring
  ];

  nixosModules.desktop = {
    openssh.enable = true;
    fonts.enable = true;
    steam.enable = true;
    bluetooth.enable = true;
  };

  services.hardware.openrgb = {
    enable = false;
    package = pkgs.openrgb-with-all-plugins;
    motherboard = "intel";
  };

  services = {
    displayManager = {
      sddm = {
        enable = true;
        wayland.enable = true;
        # wayland.compositor = "kwin";
        autoNumlock = true;
        package = pkgs.kdePackages.sddm;
      };
      gdm = {
        enable = false;
        wayland = true;
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
    udisks2.enable = true;
    devmon.enable = true;
    gvfs.enable = true;
  };

  programs.nix-ld.enable = true;
  programs.appimage = {
    enable = true;
    binfmt = true;
  };

  programs.evolution.enable = true;
  programs.sniffnet.enable = true;

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

  programs.hyprland = {
    enable = true;
  };

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
      # Audio
      pwvucontrol
      alsa-utils
      pavucontrol
      pulseaudio
    ];
  };
  networking.hostName = "${hostname}";
}
