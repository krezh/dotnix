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
        "ai"
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
          inherit (cfg.bar)
            left
            middle
            right
            spacing
            ;
        };
        appearance = {
          inherit (cfg.appearance)
            opacity
            radius
            power
            border
            shadow
            ;
          icon_theme = cfg.appearance.iconTheme;
        };
        features = {
          notifications = {
            privacy = {
              inherit (cfg.features.notifications.privacy) private;
              private_apps = cfg.features.notifications.privacy.privateApps;
              hide_on_lock = cfg.features.notifications.privacy.hideOnLock;
            };
            history_days = cfg.features.notifications.historyDays;
          };
          visualizer = {
            inherit (cfg.features.visualizer) glow;
          };
          ai = {
            inherit (cfg.features.ai) sources;
            refresh_minutes = cfg.features.ai.refreshMinutes;
            disabled_limits = cfg.features.ai.disabledLimits;
            widget = {
              inherit (cfg.features.ai.widget) display;
              show_badge = cfg.features.ai.widget.showBadge;
            };
            alerts = {
              inherit (cfg.features.ai.alerts) notifications;
              high_usage_threshold = cfg.features.ai.alerts.highUsageThreshold;
              pace_alerts = cfg.features.ai.alerts.paceAlerts;
            };
            sessions = {
              inherit (cfg.features.ai.sessions) enable;
              answer_seconds = cfg.features.ai.sessions.answerSeconds;
            };
            chat = {
              inherit (cfg.features.ai.chat) command model;
            };
          };
          session = {
            inherit (cfg.features.session) actions confirm commands;
          };
          launcher = {
            inherit (cfg.features.launcher) terminal;
          };
          clipboard = {
            history = {
              inherit (cfg.features.clipboard.history) images;
              max_entries = cfg.features.clipboard.history.maxEntries;
              max_total_mb = cfg.features.clipboard.history.maxTotalMb;
              max_item_mb = cfg.features.clipboard.history.maxItemMb;
            };
            inherit (cfg.features.clipboard) persist;
            paste = {
              inherit (cfg.features.clipboard.paste) keys terminals;
              terminal_keys = cfg.features.clipboard.paste.terminalKeys;
            };
          };
          system.storage = {
            inherit (cfg.features.system.storage) partitions;
            hidden_partitions = cfg.features.system.storage.hiddenPartitions;
          };
        };
        log = {
          inherit (cfg.log) level;
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
            "ai"
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

        appearance.opacity = lib.mkOption {
          type = lib.types.numbers.between 0 1;
          default = 0.45;
          description = "Opacity of the island body.";
        };

        appearance.radius = lib.mkOption {
          type = lib.types.numbers.nonnegative;
          default = 28;
          description = "Corner radius in pixels.";
        };

        appearance.power = lib.mkOption {
          type = lib.types.numbers.positive;
          default = 2;
          description = "Corner superellipse power: 2 is circular, 4 a squircle.";
        };

        appearance.border = lib.mkOption {
          type = lib.types.numbers.nonnegative;
          default = 1;
          description = "Width of the outline around the island, popovers and cards, in pixels; 0 turns it off.";
        };

        appearance.shadow = lib.mkOption {
          type = lib.types.numbers.nonnegative;
          default = 1;
          description = "Strength of the island's drop shadow; 0 turns it off.";
        };

        appearance.iconTheme = lib.mkOption {
          type = lib.types.str;
          default = if config.gtk.iconTheme != null then config.gtk.iconTheme.name else "hicolor";
          defaultText = lib.literalExpression ''config.gtk.iconTheme.name or "hicolor"'';
          description = "Icon theme used for tray icons, app icons and bar glyphs.";
        };

        log.level = lib.mkOption {
          type = lib.types.str;
          default = "info";
          example = "audio=debug,tray=trace";
          description = "What island logs: a level (error, warn, info, debug, trace) or per module or feature like audio=debug. Follow it with `island logs`.";
        };

        features.visualizer.glow = lib.mkOption {
          type = lib.types.numbers.between 0 1;
          default = 0.5;
          description = "Strength of the music glow behind the bar while media plays; 0 turns it off.";
        };

        features.system.storage = {
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

        features.session = {
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

        features.ai = {
          sources = lib.mkOption {
            type = lib.types.listOf (
              lib.types.enum [
                "omp"
                "claude"
              ]
            );
            default = [
              "omp"
              "claude"
            ];
            description = ''
              AI usage harnesses, in overview and tooltip order. Each harness has its own
              account tiles, refresh status and issues. Accounts without quotas or saved
              reset credits are hidden, along with harnesses that have no visible accounts;
              accounts that do not expose usage are omitted, not treated as refresh errors.
              Actual refresh and credential failures remain available as diagnostics.
              omp runs `omp usage --json` from PATH; claude reads Claude Code's existing
              OAuth login without renewing or modifying it. Successful Claude reports are
              saved per account and organization in Island's encrypted database, without
              credentials. A matching local identity restores the last report on startup;
              environment-token logins need an online profile lookup to establish identity.
              Restored reports are marked cached until verified, or stale after a failed
              refresh, with their original fetch time. Passed reset times remain unverified;
              cached data never implies a quota reset or generates usage alerts.
              Expired-login warnings are logged once until the failure changes or clears.
              Credential renewal is picked up on the next poll or `island ctl ai refresh`.
              Empty disables collection.
            '';
          };
          disabledLimits = lib.mkOption {
            type = lib.types.submodule {
              options = lib.genAttrs [ "omp" "claude" ] (
                source:
                lib.mkOption {
                  type = lib.types.attrsOf (lib.types.listOf lib.types.str);
                  default = { };
                  description = "Quota IDs disabled for ${source}, keyed by provider ID.";
                }
              );
            };
            default = { };
            example = {
              omp.google-antigravity = [ "3p-weekly:weekly" ];
            };
            description = ''
              Quotas excluded from the overview, widget, tooltip and notifications.
              Rules apply to every account of the selected provider within that harness.
              Shared quota pools use their scope.sharedGroup ID; other quotas use their
              limit ID from `omp usage --json`. Claude Code uses provider ID `anthropic`
              and OAuth window IDs such as `five_hour` and `seven_day`.
              The example disables Antigravity's shared Claude/GPT pool, leaving Gemini
              and standalone Claude quotas enabled.
            '';
          };
          refreshMinutes = lib.mkOption {
            type = lib.types.ints.between 1 60;
            default = 5;
            description = "Minutes between AI quota refreshes.";
          };
          alerts.highUsageThreshold = lib.mkOption {
            type = lib.types.numbers.between 50 99;
            default = 75;
            description = "Quota percentage at which healthy usage becomes a high-usage alert.";
          };
          alerts.paceAlerts = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Warn when the current usage rate would exhaust a quota before its reset.";
          };
          alerts.notifications = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Notify when AI quotas become high, exhausted or available again.";
          };
          widget.display = lib.mkOption {
            type = lib.types.enum [
              "worst"
              "providers"
            ];
            default = "worst";
            description = "Show the most urgent quota alert or each provider's headline usage in the AI bar widget. Each badge uses the provider icon of the account supplying its quota or countdown.";
          };
          widget.showBadge = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Show the reset countdown or usage badge in the AI widget's worst-alert mode.";
          };
          sessions.enable = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Follow Claude Code sessions through their hooks and answer their permission requests and questions from island.";
          };
          sessions.answerSeconds = lib.mkOption {
            type = lib.types.ints.between 5 3600;
            default = 60;
            description = "Seconds a request waits for an answer in island before it goes back to the terminal. Keep it below the hook's own timeout.";
          };
          chat.command = lib.mkOption {
            type = lib.types.str;
            default = "claude";
            description = "Claude Code command the chat runs, by name on the PATH or as a full path.";
          };
          chat.model = lib.mkOption {
            type = lib.types.enum [
              "haiku"
              "sonnet"
              "opus"
            ];
            default = "sonnet";
            description = "Model a new chat starts with.";
          };
        };

        features.launcher.terminal = lib.mkOption {
          type = lib.types.nonEmptyListOf lib.types.str;
          default = [ "xdg-terminal-exec" ];
          example = [
            "foot"
            "-e"
          ];
          description = ''
            Terminal command (argv) prepended to desktop entries with Terminal=true.
            Arguments are kept verbatim, without shell parsing; ordinary apps are unchanged.
            The equivalent island TOML is:
            ```toml
            [features.launcher]
            terminal = ["foot", "-e"]
            ```
          '';
        };

        features.clipboard = {
          history.maxEntries = lib.mkOption {
            type = lib.types.ints.positive;
            default = 200;
            description = "Clipboard history entries kept, most recently used first.";
          };
          history.maxTotalMb = lib.mkOption {
            type = lib.types.ints.positive;
            default = 256;
            description = "Megabytes the whole clipboard history may take.";
          };
          history.maxItemMb = lib.mkOption {
            type = lib.types.ints.positive;
            default = 32;
            description = "Copies larger than this many megabytes are not kept.";
          };
          history.images = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Keep copied pictures as well as text and files.";
          };
          persist = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Put the newest entry back on the clipboard when it empties, such as when the app that copied something quits.";
          };
          paste.keys = lib.mkOption {
            type = lib.types.str;
            default = "ctrl+v";
            description = "Keys pressed in the focused window to paste a picked entry; modifiers ctrl, shift, alt, super and a letter or insert. Empty only puts the entry on the clipboard.";
          };
          paste.terminalKeys = lib.mkOption {
            type = lib.types.str;
            default = "ctrl+shift+v";
            description = "Keys pressed instead of paste.keys when the focused window is one of paste.terminals.";
          };
          paste.terminals = lib.mkOption {
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
            description = "Window classes of terminals, which paste with terminalKeys.";
          };
        };

        features.notifications = {
          privacy.private = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Hide notification contents by default.";
          };
          privacy.privateApps = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            example = [ "Signal" ];
            description = "Apps whose notification contents are always hidden.";
          };
          privacy.hideOnLock = lib.mkOption {
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
