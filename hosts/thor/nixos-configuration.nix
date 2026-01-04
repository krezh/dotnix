{
  pkgs,
  inputs,
  lib,
  hostname,
  ...
}:
{
  imports = [
    inputs.niri.nixosModules.niri
  ];

  nixpkgs.overlays = [
    inputs.niri.overlays.niri
    inputs.nix-cachyos-kernel.overlay
  ];

  programs.hyprland = {
    enable = true;
  };

  programs.niri.enable = true;
  programs.niri.package = pkgs.niri-unstable;

  boot = {
    plymouth = {
      enable = true;
    };
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

  services.fwupd.enable = true;
  services.accounts-daemon.enable = true;
  services.gnome.gnome-online-accounts.enable = true;

  services.scx.enable = false;
  services.scx.scheduler = "scx_bpfland";

  xdg.mime.enable = true;

  # Disable Coredump
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

  nixosModules = {
    openssh.enable = true;
    fonts.enable = true;
    steam.enable = true;
    bluetooth.enable = true;
    claude-code.enable = true;
  };

  services.hardware.openrgb = {
    enable = false;
    package = pkgs.openrgb-with-all-plugins;
    motherboard = "amd";
  };

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
  security.rtkit.enable = true;

  programs.nix-ld.enable = true;
  programs.appimage = {
    enable = true;
    binfmt = true;
  };

  programs.evolution.enable = true;
  programs.sniffnet.enable = true;

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
