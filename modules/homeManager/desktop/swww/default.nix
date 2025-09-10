{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.swww;

  swww-random = pkgs.buildGoModule rec {
    pname = "swww-random";
    version = "1.0.0";
    src = ./src;
    vendorHash = null;
    meta = with lib; {
      description = "Random wallpaper setter for swww";
      homepage = "https://github.com/Horus645/swww";
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

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = {
        transitionFPS = 60;
        transitionStep = 120;
        transition = "grow";
        transitionPos = "center";
        interval = 300;
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
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
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
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
