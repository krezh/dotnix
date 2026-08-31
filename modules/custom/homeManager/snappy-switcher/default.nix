{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.homeModules.snappy-switcher;
      iniFormat = pkgs.formats.ini { };
      mkBinds = lib.mapAttrsToList (
        keys:
        { rule, ... }@opts:
        {
          _args = [
            keys
            rule
            (removeAttrs opts [ "rule" ])
          ];
        }
      );
      exec = cmd: lib.generators.mkLuaInline "hl.dsp.exec_cmd(${builtins.toJSON cmd})";
    in
    {
      options.homeModules.snappy-switcher = {
        enable = lib.mkEnableOption "snappy-switcher Alt+Tab window switcher for Hyprland";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.snappy-switcher;
          description = "snappy-switcher derivation to use.";
        };

        settings = lib.mkOption {
          inherit (iniFormat) type;
          default = {
            general = {
              mode = "context";
              follow_monitor = true;
              show_workspace_badge = true;
              sticky_mode = false;
              ignore_special = true;
            };
            theme.name = "catppuccin-mocha.ini";
          };
          description = "snappy-switcher config.ini settings.";
        };
      };

      config = lib.mkIf cfg.enable {
        home.packages = [ cfg.package ];

        xdg.configFile = {
          "snappy-switcher/config.ini".source = iniFormat.generate "snappy-switcher-config.ini" cfg.settings;
          "snappy-switcher/themes".source = "${cfg.package}/share/snappy-switcher/themes";
        };

        systemd.user.services.snappy-switcher = {
          Unit = {
            Description = "Snappy Switcher - Alt+Tab window switcher for Hyprland";
            After = [ "graphical-session.target" ];
            PartOf = [ "graphical-session.target" ];
          };
          Service = {
            ExecStart = "${lib.getExe cfg.package} --daemon";
            ExecStop = "${lib.getExe cfg.package} quit";
            Restart = "on-failure";
            RestartSec = 3;
          };
          Install.WantedBy = [ "graphical-session.target" ];
        };

        wayland.windowManager.hyprland.settings.bind = mkBinds {
          "ALT + TAB" = {
            rule = exec "${lib.getExe cfg.package} next --mod alt";
            desc = "Switch window";
          };
        };
      };
    };
}
