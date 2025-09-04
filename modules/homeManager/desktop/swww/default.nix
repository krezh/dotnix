{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.swww;
  swww-random = pkgs.writeScriptBin "swww-random" (builtins.readFile ./scripts/swww-random);
in
{
  options.hmModules.desktop.swww = {
    enable = lib.mkEnableOption "swww";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.swww;
      description = ''
        swww derivation to use.
      '';
    };

    path = lib.mkOption {
      type = lib.types.path;
      default = "$HOME/wallpapers";
    };

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = {
        transitionFPS = 60;
        transitionStep = 120;
        transition = "grow";
        transitionPos = "center";
        interval = 300;
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home = {
      packages = [ cfg.package ];
      sessionVariables = {
        SWWW_TRANSITION_FPS = "${toString cfg.settings.transitionFPS}";
        SWWW_TRANSITION_STEP = "${toString cfg.settings.transitionStep}";
        SWWW_TRANSITION = "${cfg.settings.transition}";
        SWWW_TRANSITION_POS = "${cfg.settings.transitionPos}";
      };
    };

    systemd.user.services.swww-daemon = {
      Unit = {
        Description = "A Solution to your Wayland Wallpaper Woes";
        Documentation = "https://github.com/Horus645/swww";
        After = [ "graphical-session.target" ];
        Requires = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/swww-daemon -q";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };

    systemd.user.services.swww-random = {
      Unit = {
        Description = "A Solution to your Wayland Wallpaper Woes";
        Documentation = "https://github.com/Horus645/swww";
        After = [ "swww-daemon.service" ];
        Requires = [
          "swww-daemon.service"
          "graphical-session.target"
        ];
      };
      Service = {
        Environment = [
          "SWWW_TRANSITION_FPS=${toString cfg.settings.transitionFPS}"
          "SWWW_TRANSITION_STEP=${toString cfg.settings.transitionStep}"
          "SWWW_TRANSITION=${cfg.settings.transition}"
          "SWWW_TRANSITION_POS=${cfg.settings.transitionPos}"
        ];
        ExecStart = "${swww-random}/bin/swww-random -d ${cfg.path} -i ${toString cfg.settings.interval}";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
