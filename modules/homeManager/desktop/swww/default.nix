{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.swww;

  swww-random = pkgs.buildGo125Module rec {
    pname = "swww-random";
    version = "1.0.0";
    src = ./swww-random;
    vendorHash = null;
    meta = with lib; {
      description = "Random wallpaper setter for swww";
      license = licenses.gpl3;
      platforms = platforms.linux;
      mainProgram = pname;
    };
  };
in
{
  options.hmModules.desktop.swww = {
    enable = lib.mkEnableOption "swww";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.swww;
      description = ''
        swww derivation to use.
      '';
    };

    path = lib.mkOption {
      type = lib.types.path;
      default = "$HOME/wallpapers";
    };

    settings = {
      transitionFPS = lib.mkOption {
        type = lib.types.int;
        default = 60;
        description = "Repository type for Kopia backups";
      };
      transitionStep = lib.mkOption {
        type = lib.types.int;
        default = 120;
        description = "Number of steps in the transition animation";
      };
      transition = lib.mkOption {
        type = lib.types.enum [
          "fade"
          "grow"
          "slide_left"
          "slide_right"
          "slide_up"
          "slide_down"
          "instant"
        ];
        default = "grow";
        description = "Transition effect to use when changing wallpapers";
      };
      transitionPos = lib.mkOption {
        type = lib.types.enum [
          "center"
          "top"
          "bottom"
          "left"
          "right"
          "top_left"
          "top_right"
          "bottom_left"
          "bottom_right"
        ];
        default = "center";
        description = "Position of the wallpaper on the screen";
      };
      interval = lib.mkOption {
        type = lib.types.int;
        default = 300;
        description = "Interval in seconds between wallpaper changes";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home = {
      packages = [
        cfg.package
        swww-random
      ];
      sessionVariables = {
        SWWW_TRANSITION_FPS = "${toString cfg.settings.transitionFPS}";
        SWWW_TRANSITION_STEP = "${toString cfg.settings.transitionStep}";
        SWWW_TRANSITION = "${cfg.settings.transition}";
        SWWW_TRANSITION_POS = "${cfg.settings.transitionPos}";
      };
    };

    systemd.user.services.swww-daemon = {
      Unit = {
        Description = "A Solution to your Wayland Wallpaper Woes";
        Documentation = "https://github.com/Horus645/swww";
        After = [ "graphical-session.target" ];
        Requires = [ "graphical-session.target" ];
      };
      Service = {
        Type = "simple";
        ExecStartPre = "${pkgs.bash}/bin/bash -c '${cfg.package}/bin/swww kill 2>/dev/null || true'";
        ExecStart = "${cfg.package}/bin/swww-daemon";
        ExecStop = "${cfg.package}/bin/swww kill";
        Restart = "on-failure";
        RestartSec = 5;
        RemainAfterExit = false;
      };
      Install.WantedBy = [
        "graphical-session.target"
      ];
    };

    systemd.user.services.swww-random = {
      Unit = {
        Description = "Random wallpaper setter for swww";
        Documentation = "https://github.com/Horus645/swww";
        After = [ "swww-daemon.service" ];
        Requires = [
          "swww-daemon.service"
          "graphical-session.target"
        ];
      };
      Service = {
        Environment = [
          "SWWW_TRANSITION_FPS=${toString cfg.settings.transitionFPS}"
          "SWWW_TRANSITION_STEP=${toString cfg.settings.transitionStep}"
          "SWWW_TRANSITION=${cfg.settings.transition}"
          "SWWW_TRANSITION_POS=${cfg.settings.transitionPos}"
        ];
        ExecStart = "${lib.getExe swww-random} -d ${cfg.path} -i ${toString cfg.settings.interval}";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [
        "graphical-session.target"
      ];
    };
  };
}
