{ inputs, lib, pkgs, config, ... }:
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
  ];

  guiDeps = with pkgs; [
    gnome.gnome-control-center
    mission-center
    overskride
    wlogout
  ];

  dependencies = requiredDeps ++ guiDeps;

  cfg = config.programs.gBar;
in {
  imports = [ inputs.gBar.homeManagerModules.x86_64-linux.default ];

  programs.gBar = {
    enable = true;
    config = {
      Location = "T";
      EnableSNI = true;
      SNIIconSize = {
        Discord = 26;
        OBS = 23;
      };
      WorkspaceSymbols = [ " " " " ];
    };
  };
  systemd.user.services.gbar = {
    Unit = {
      Description = "gBar";
      PartOf = [ "tray.target" "graphical-session.target" ];
    };
    Service = {
      Environment = "PATH=/run/wrappers/bin:${lib.makeBinPath dependencies}";
      ExecStart = "${cfg.package}/bin/gBar bar eDP-1";
      Restart = "on-failure";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
