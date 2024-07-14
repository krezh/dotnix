{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.modules.desktop.swww;
  swww-random = pkgs.writeScriptBin "swww-random" (builtins.readFile ./scripts/swww-random);
  inherit (pkgs) swww;
in
{
  options.modules.desktop.swww = {
    enable = lib.mkEnableOption "swww";

    package = lib.mkPackageOption pkgs swww { };

    interval = lib.mkOption {
      type = lib.types.ints.positive;
      default = 300;
    };

    path = lib.mkOption {
      type = lib.types.path;
      default = "$HOME/wallpapers";
    };

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = { };
    };
  };

  config = lib.mkIf cfg.enable {
    home = {
      packages = [
        swww
        swww-random
      ];
      sessionVariables = {
        SWWW_TRANSITION_FPS = 60;
        SWWW_TRANSITION_STEP = 2;
        SWWW_TRANSITION_TYPE = "grow";
        SWWW_TRANSITION_POS = "center";
      };
    };

    systemd.user.services.swww-daemon = {
      Unit = {
        Description = "A Solution to your Wayland Wallpaper Woes";
        Documentation = "https://github.com/Horus645/swww";
      };
      Service = {
        PassEnvironment = [
          # "HOME"
          "PATH"
          "XDG_RUNTIME_DIR"
          "SWWW_TRANSITION_TYPE"
          "SWWW_TRANSITION_STEP"
          "SWWW_TRANSITION_FPS"
          "SWWW_TRANSITION_BEZIER"
        ];
        ExecStart = "${swww}/bin/swww-daemon";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };

    systemd.user.services.swww-random = {
      Unit = {
        Description = "A Solution to your Wayland Wallpaper Woes";
        Documentation = "https://github.com/Horus645/swww";
        Requires = [
          "swww-daemon.service"
          "graphical-session.target"
        ];
      };
      Service = {
        PassEnvironment = [ "PATH" ];
        ExecStart = "${swww-random}/bin/swww-random ${cfg.path} ${toString cfg.interval}";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };

  };
}
