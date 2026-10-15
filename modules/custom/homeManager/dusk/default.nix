{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.homeModules.dusk;
      tomlFormat = pkgs.formats.toml { };
      actionDefaults = {
        logout = {
          topLabel = "Logging out...";
          command = "";
        };
        shutdown = {
          topLabel = "Shutting down...";
          command = "${lib.getExe' pkgs.systemd "systemctl"} poweroff";
        };
        reboot = {
          topLabel = "Restarting...";
          command = "${lib.getExe' pkgs.systemd "systemctl"} reboot";
        };
      };
    in
    {
      options.homeModules.dusk = {
        enable = lib.mkEnableOption "Dusk graceful session exit for Hyprland";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.dusk;
          description = "Dusk package to use.";
        };

        dryRun = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Preview without closing applications, exiting Hyprland, switching VT, or running the action command.";
        };

        noExit = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Close applications without exiting Hyprland, switching VT, or running the action command.";
        };

        actions = lib.mapAttrs (action: defaults: {
          topLabel = lib.mkOption {
            type = lib.types.str;
            default = defaults.topLabel;
            description = "Heading shown for ${action}; --top-label overrides it for an invocation.";
          };

          command = lib.mkOption {
            type = lib.types.str;
            default = defaults.command;
            description = ''
              Shell command for ${action}, run only after graceful application closure and
              confirmed Hyprland exit, never on cancellation, dry-run, no-exit, or errors.
              An empty string disables the additional command, not Hyprland exit.
              Logout defaults to no additional command; shutdown and reboot default to
              the absolute systemctl poweroff and reboot commands, respectively.
              --command overrides it for an invocation.
            '';
          };
        }) actionDefaults;

        verbose = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Print session exit diagnostics.";
        };

        noFork = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Run in the foreground instead of detaching.";
        };

        vt = lib.mkOption {
          type = lib.types.nullOr (lib.types.ints.between 1 63);
          default = null;
          description = "Virtual terminal to switch to using sudo -n chvt after Hyprland exits. Null disables switching.";
        };

        opacity = lib.mkOption {
          type = lib.types.numbers.between 0 1;
          default = 0.75;
          description = "Main panel opacity. The fullscreen background remains transparent.";
        };

        panelOpacity = lib.mkOption {
          type = lib.types.numbers.between 0 1;
          default = 0.60;
          description = "Inner panel opacity, composited over the main panel.";
        };
      };

      config = lib.mkIf cfg.enable {
        home.packages = [ cfg.package ];

        xdg.configFile."dusk/config.toml".source = tomlFormat.generate "dusk-config.toml" (
          {
            dry_run = cfg.dryRun;
            no_exit = cfg.noExit;
            verbose = cfg.verbose;
            no_fork = cfg.noFork;
            actions = lib.mapAttrs (_: action: {
              top_label = action.topLabel;
              command = action.command;
            }) cfg.actions;
            appearance = {
              opacity = cfg.opacity;
              panel_opacity = cfg.panelOpacity;
            };
          }
          // lib.optionalAttrs (cfg.vt != null) { vt = cfg.vt; }
        );
      };
    };
}
