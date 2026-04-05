{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.services.awww-random;

      awww-random = pkgs.buildGoModule rec {
        pname = "awww-random";
        version = "1.0.0";
        src = ./src;
        vendorHash = null;
        meta = with lib; {
          description = "Random wallpaper setter for awww";
          license = licenses.gpl3;
          platforms = platforms.linux;
          mainProgram = pname;
        };
      };
    in
    {
      options.services.awww-random = {
        enable = lib.mkEnableOption "awww random wallpaper service";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.awww;
          description = "awww derivation to use.";
        };

        path = lib.mkOption {
          type = lib.types.path;
          description = "Path to wallpaper directory.";
          default = config.home.file.wallpapers.source;
        };

        settings = {
          transitionFPS = lib.mkOption {
            type = lib.types.int;
            default = 60;
            description = "Frames per second for transition animation.";
          };
          transitionStep = lib.mkOption {
            type = lib.types.int;
            default = 120;
            description = "Number of steps in the transition animation.";
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
            description = "Transition effect to use when changing wallpapers.";
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
            description = "Position for grow/slide transitions.";
          };
          interval = lib.mkOption {
            type = lib.types.int;
            default = 300;
            description = "Interval in seconds between wallpaper changes.";
          };
        };
      };

      config = lib.mkIf cfg.enable {
        home = {
          packages = [
            cfg.package
            awww-random
            pkgs.waypaper
          ];
          sessionVariables = {
            AWWW_TRANSITION_FPS = "${toString cfg.settings.transitionFPS}";
            AWWW_TRANSITION_STEP = "${toString cfg.settings.transitionStep}";
            AWWW_TRANSITION = "${cfg.settings.transition}";
            AWWW_TRANSITION_POS = "${cfg.settings.transitionPos}";
          };
        };

        home.file."wallpapers" = {
          recursive = true;
          source = ./wallpapers;
        };

        systemd.user.services.awww-daemon = {
          Unit = {
            Description = "A Solution to your Wayland Wallpaper Woes";
            Documentation = "https://github.com/Horus645/awww";
            Requires = [ "graphical-session.target" ];
          };
          Service = {
            Type = "simple";
            ExecStartPre = "${pkgs.bash}/bin/bash -c '${cfg.package}/bin/awww kill 2>/dev/null || true'";
            ExecStart = "${cfg.package}/bin/awww-daemon";
            ExecStop = "${cfg.package}/bin/awww kill";
            Restart = "on-failure";
            RestartSec = 5;
            RemainAfterExit = false;
          };
          Install.WantedBy = [ "graphical-session.target" ];
        };

        systemd.user.services.awww-random = {
          Unit = {
            Description = "Random wallpaper setter for awww";
            Requires = [ "awww-daemon.service" ];
          };
          Service = {
            Environment = [
              "AWWW_TRANSITION_FPS=${toString cfg.settings.transitionFPS}"
              "AWWW_TRANSITION_STEP=${toString cfg.settings.transitionStep}"
              "AWWW_TRANSITION=${cfg.settings.transition}"
              "AWWW_TRANSITION_POS=${cfg.settings.transitionPos}"
            ];
            ExecStart = "${lib.getExe awww-random} -d ${cfg.path} -i ${toString cfg.settings.interval}";
            Restart = "on-failure";
            RestartSec = 5;
          };
          Install.WantedBy = [ "awww-daemon.service" ];
        };
      };
    };
}
