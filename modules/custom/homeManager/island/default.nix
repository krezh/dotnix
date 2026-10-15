{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.homeModules.island;
      toml = pkgs.formats.toml { };
      widget = lib.types.enum [
        "tray"
        "workspaces"
        "clock"
        "notifications"
        "audio"
        "media"
        "molt"
        "headset"
        "zowie"
        "caffeine"
        "visualizer"
      ];
      section =
        default:
        lib.mkOption {
          type = lib.types.listOf widget;
          inherit default;
          description = "Widgets in this bar section, in order.";
        };
      configFile = toml.generate "island.toml" {
        bar = {
          inherit (cfg.bar) left middle right spacing;
        };
        appearance = {
          inherit (cfg) opacity radius power;
          icon_theme = cfg.iconTheme;
        };
        notifications = {
          inherit (cfg.notifications) private;
          private_apps = cfg.notifications.privateApps;
          hide_on_lock = cfg.notifications.hideOnLock;
        };
        visualizer = {
          inherit (cfg.visualizer) glow;
        };
        log = {
          level = cfg.logLevel;
        };
      };
    in
    {
      options.homeModules.island = {
        enable = lib.mkEnableOption "island, a dynamic island status bar and notification daemon for Hyprland";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.island;
          description = "island derivation to use.";
        };

        bar = {
          left = section [
            "tray"
            "molt"
            "headset"
            "zowie"
            "audio"
            "caffeine"
          ];
          middle = section [ "workspaces" ];
          right = section [
            "media"
            "clock"
            "notifications"
          ];
          spacing = lib.mkOption {
            type = lib.types.numbers.nonnegative;
            default = 4;
            description = "Gap between bar widget highlights in pixels.";
          };
        };

        opacity = lib.mkOption {
          type = lib.types.numbers.between 0 1;
          default = 0.45;
          description = "Opacity of the island body.";
        };

        radius = lib.mkOption {
          type = lib.types.numbers.nonnegative;
          default = 28;
          description = "Corner radius in pixels.";
        };

        power = lib.mkOption {
          type = lib.types.numbers.positive;
          default = 2;
          description = "Corner superellipse power: 2 is circular, 4 a squircle.";
        };

        iconTheme = lib.mkOption {
          type = lib.types.str;
          default = if config.gtk.iconTheme != null then config.gtk.iconTheme.name else "hicolor";
          defaultText = lib.literalExpression ''config.gtk.iconTheme.name or "hicolor"'';
          description = "Icon theme used for tray icons, app icons and bar glyphs.";
        };

        logLevel = lib.mkOption {
          type = lib.types.str;
          default = "info";
          example = "audio=debug,tray=trace";
          description = "What island logs: a level (error, warn, info, debug, trace) or per module or feature like audio=debug. Follow it with `island logs`.";
        };

        visualizer.glow = lib.mkOption {
          type = lib.types.numbers.between 0 1;
          default = 0.5;
          description = "Strength of the music glow behind the bar while media plays; 0 turns it off.";
        };

        notifications = {
          private = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Hide notification contents by default.";
          };
          privateApps = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            example = [ "Signal" ];
            description = "Apps whose notification contents are always hidden.";
          };
          hideOnLock = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Hide notification contents while the session is locked.";
          };
        };
      };

      config = lib.mkIf cfg.enable {
        home.packages = [ cfg.package ];

        xdg.configFile."island/config.toml".source = configFile;

        systemd.user.services.island = {
          Unit = {
            Description = "island dynamic island status bar";
            After = [ "graphical-session.target" ];
            PartOf = [ "graphical-session.target" ];
            ConditionEnvironment = "HYPRLAND_INSTANCE_SIGNATURE";
            X-Restart-Triggers = [ "${configFile}" ];
            StartLimitIntervalSec = 0;
          };
          Service = {
            ExecStart = "${lib.getExe cfg.package} --config ${config.xdg.configHome}/island/config.toml";
            Restart = "on-failure";
            RestartSec = 1;
            RestartSteps = 5;
            RestartMaxDelaySec = 30;
          };
          Install.WantedBy = [ "graphical-session.target" ];
        };
      };
    };
}
