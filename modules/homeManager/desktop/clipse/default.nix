{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.clipse;
  json = pkgs.formats.json { };
  defaultConfig = {
    historyFile = "clipboard_history.json";
    maxHistory = 100;
    allowDuplicates = false;
    themeFile = "custom_theme.json";
    tempDir = "tmp_files";
    logFile = "clipse.log";
    keyBindings = {
      choose = "enter";
      clearSelected = "S";
      down = "down";
      end = "end";
      filter = "/";
      home = "home";
      more = "?";
      nextPage = "right";
      prevPage = "left";
      preview = "t";
      quit = "q";
      remove = "x";
      selectDown = "ctrl+down";
      selectSingle = "s";
      selectUp = "ctrl+up";
      togglePin = "p";
      togglePinned = "tab";
      up = "up";
      yankFilter = "ctrl+s";
    };
    imageDisplay = {
      type = "basic";
      scaleX = 9;
      scaleY = 9;
      heightCut = 2;
    };
  };
in
{
  options.hmModules.desktop.clipse = {
    enable = lib.mkEnableOption "clipse";

    package = lib.mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.clipse;
    };

    config = lib.mkOption {
      type = lib.types.attrs;
      default = { };
    };
  };

  config = lib.mkIf cfg.enable {

    home.packages = [
      cfg.package
      pkgs.wl-clipboard
    ];

    xdg.configFile."clipse/config.json" = {
      source = json.generate "clipse/config.json" (lib.recursiveUpdate defaultConfig cfg.config);
    };

    systemd.user.services.clipse = {
      Unit = {
        Description = "Configurable TUI clipboard manager for Unix";
        Documentation = "https://github.com/savedra1/clipse";
        X-SwitchMethod = "restart";
      };
      Service = {
        PassEnvironment = [
          "PATH"
          "XDG_RUNTIME_DIR"
        ];
        ExecStart = "${cfg.package}/bin/clipse --listen-shell";
        Restart = "always";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
