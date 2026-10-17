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
        "session"
        "clipboard"
        "visualizer"
      ];
      sessionActions = [
        "lock"
        "logout"
        "suspend"
        "hibernate"
        "reboot"
        "shutdown"
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
          history_days = cfg.notifications.historyDays;
        };
        visualizer = {
          inherit (cfg.visualizer) glow;
        };
        session = {
          inherit (cfg.session) actions confirm commands;
        };
        clipboard = {
          inherit (cfg.clipboard)
            images
            persist
            paste
            terminals
            ;
          terminal_paste = cfg.clipboard.terminalPaste;
          max_entries = cfg.clipboard.maxEntries;
          max_total_mb = cfg.clipboard.maxTotalMb;
          max_item_mb = cfg.clipboard.maxItemMb;
        };
        system = {
          inherit (cfg.system) partitions;
          hidden_partitions = cfg.system.hiddenPartitions;
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
            "session"
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

        system = {
          partitions = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            example = [
              "/"
              "/home"
            ];
            description = "Mount points the System panel's Storage tile shows, in this order; empty shows every mounted partition not in hiddenPartitions.";
          };
          hiddenPartitions = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ "/boot" ];
            description = "Mount points the Storage tile leaves out when partitions is empty.";
          };
        };

        session = {
          actions = lib.mkOption {
            type = lib.types.listOf (lib.types.enum sessionActions);
            default = [
              "lock"
              "logout"
              "suspend"
              "reboot"
              "shutdown"
            ];
            description = "Buttons in the Session panel, in order.";
          };
          confirm = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Whether logout, reboot and shutdown from the Session panel need a second click. `island ctl session` never asks.";
          };
          commands = lib.genAttrs sessionActions (
            action:
            lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = "Command (argv) run for ${action} instead of asking logind; empty asks logind.";
            }
          );
        };

        clipboard = {
          maxEntries = lib.mkOption {
            type = lib.types.ints.positive;
            default = 200;
            description = "Clipboard history entries kept, most recently used first.";
          };
          maxTotalMb = lib.mkOption {
            type = lib.types.ints.positive;
            default = 256;
            description = "Megabytes the whole clipboard history may take.";
          };
          maxItemMb = lib.mkOption {
            type = lib.types.ints.positive;
            default = 32;
            description = "Copies larger than this many megabytes are not kept.";
          };
          images = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Keep copied pictures as well as text and files.";
          };
          persist = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Put the newest entry back on the clipboard when it empties, such as when the app that copied something quits.";
          };
          paste = lib.mkOption {
            type = lib.types.str;
            default = "ctrl+v";
            description = "Keys pressed in the focused window to paste a picked entry; modifiers ctrl, shift, alt, super and a letter or insert. Empty only puts the entry on the clipboard.";
          };
          terminalPaste = lib.mkOption {
            type = lib.types.str;
            default = "ctrl+shift+v";
            description = "Keys pressed instead of paste when the focused window is one of terminals.";
          };
          terminals = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [
              "com.mitchellh.ghostty"
              "kitty"
              "foot"
              "footclient"
              "Alacritty"
              "org.wezfurlong.wezterm"
              "org.gnome.Ptyxis"
              "org.gnome.Console"
              "org.kde.konsole"
              "xterm"
            ];
            description = "Window classes of terminals, which paste with terminalPaste.";
          };
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
          historyDays = lib.mkOption {
            type = lib.types.ints.positive;
            default = 7;
            description = "How many days the notification history, the panel's Earlier view, keeps notifications.";
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
