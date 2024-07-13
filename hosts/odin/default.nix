# This is your system's configuration file.
{ inputs, pkgs, ... }:
let
  tuigreet = "${pkgs.greetd.tuigreet}/bin/tuigreet";
  hyprland-session = "${inputs.hyprland.packages.${pkgs.system}.hyprland}/share/wayland-sessions";
in
{
  imports = [
    inputs.hyprland.nixosModules.default
    inputs.hardware.nixosModules.common-cpu-intel

    ../common/global
    ../common/users/krezh
    ./diskconfig.nix
    ./hardware-configuration.nix
  ];

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
  networking.networkmanager.enable = true;

  programs.gdk-pixbuf.modulePackages = [ pkgs.librsvg ];

  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${tuigreet} --time --remember --remember-session --sessions ${hyprland-session}";
        user = "greeter";
      };
    };
  };

  # this is a life saver.
  # literally no documentation about this anywhere.
  # might be good to write about this...
  # https://www.reddit.com/r/NixOS/comments/u0cdpi/tuigreet_with_xmonad_how/
  systemd.services.greetd.serviceConfig = {
    Type = "idle";
    StandardInput = "tty";
    StandardOutput = "tty";
    StandardError = "journal"; # Without this errors will spam on screen
    # Without these bootlogs will spam on screen
    TTYReset = true;
    TTYVHangup = true;
    TTYVTDisallocate = true;
  };

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

  # rtkit is optional but recommended
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    # If you want to use JACK applications, uncomment this
    jack.enable = true;
  };

  services.upower.enable = true;

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
