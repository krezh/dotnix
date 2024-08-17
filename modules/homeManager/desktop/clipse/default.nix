{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.clipse;
  json = pkgs.formats.json { };
in
{
  options.hmModules.desktop.clipse = {
    enable = lib.mkEnableOption "clipse";

    package = lib.mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.clipse;
    };

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = {
        allowDuplicates = false;
        historyFile = "clipboard_history.json";
        logFile = "clipse.log";
        maxHistory = 20;
        tempDir = "tmp_files";
        themeFile = "custom_theme.json";
      };
    };
  };

  config = lib.mkIf cfg.enable {

    home.packages = [ cfg.package ];

    wayland.windowManager.hyprland = {
      settings = {
        bind = [ "$mainMod,C,exec,${cfg.package}/bin/clipse" ];
      };
    };

    xdg.configFile."clipse/config.json" = lib.mkIf (cfg.settings != { }) {
      source = json.generate "config.json" cfg.settings;
    };

    systemd.user.services.clipse = {
      Unit = {
        Description = "Configurable TUI clipboard manager for Unix";
        Documentation = "https://github.com/savedra1/clipse";
      };
      Service = {
        PassEnvironment = [
          "PATH"
          "XDG_RUNTIME_DIR"
        ];
        ExecStart = "${cfg.package}/bin/clipse --listen-shell";
        Restart = "on-failure";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
