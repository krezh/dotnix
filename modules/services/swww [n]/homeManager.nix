{
  flake.modules.homeManager.swww =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.services.swww-random;

      swww-random = pkgs.buildGoModule rec {
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
      options.services.swww-random = {
        enable = lib.mkEnableOption "swww random wallpaper service";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.swww;
          description = "swww derivation to use.";
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
            swww-random
            pkgs.waypaper
          ];
          sessionVariables = {
            SWWW_TRANSITION_FPS = "${toString cfg.settings.transitionFPS}";
            SWWW_TRANSITION_STEP = "${toString cfg.settings.transitionStep}";
            SWWW_TRANSITION = "${cfg.settings.transition}";
            SWWW_TRANSITION_POS = "${cfg.settings.transitionPos}";
          };
        };

        home.file."wallpapers" = {
          recursive = true;
          source = ./wallpapers;
        };

        systemd.user.services.swww-daemon = {
          Unit = {
            Description = "A Solution to your Wayland Wallpaper Woes";
            Documentation = "https://github.com/Horus645/swww";
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
          Install.WantedBy = [ "graphical-session.target" ];
        };

        systemd.user.services.swww-random = {
          Unit = {
            Description = "Random wallpaper setter for swww";
            Requires = [ "swww-daemon.service" ];
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
          Install.WantedBy = [ "swww-daemon.service" ];
        };
      };
    };
}
