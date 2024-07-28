{
  inputs,
  pkgs,
  lib,
  config,
  ...
}:
let
  requiredDeps = with pkgs; [
    config.wayland.windowManager.hyprland.package
    bash
    coreutils
    dart-sass
    gawk
    imagemagick
    procps
    ripgrep
    util-linux
    pipewire
    bluez
    bluez-tools
    grimblast
    gpu-screen-recorder
    hyprpicker
    btop
    networkmanager
    dart-sass
    brightnessctl
    bun
  ];

  guiDeps = with pkgs; [
    gnome.gnome-control-center
    mission-center
    overskride
    wlogout
    gnome.gnome-bluetooth
  ];

  dependencies = requiredDeps ++ guiDeps;

  cfg = config.programs.ags;
in
{
  imports = [ inputs.ags.homeManagerModules.default ];

  programs.ags = {
    enable = true;
    configDir = ./config;
  };

  systemd.user.services.ags = {
    Unit = {
      Description = "Aylur's Gtk Shell";
      PartOf = [
        "tray.target"
        "graphical-session.target"
      ];
    };
    Service = {
      Environment = "PATH=/run/wrappers/bin:${lib.makeBinPath dependencies}";
      ExecStart = "${cfg.package}/bin/ags";
      Restart = "on-failure";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
