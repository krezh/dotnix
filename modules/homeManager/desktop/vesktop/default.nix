{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hmModules.desktop.vesktop;
  json = pkgs.formats.json { };
  inherit (lib) mkEnableOption mkOption mkIf;
in
{
  options.hmModules.desktop.vesktop = {
    enable = mkEnableOption "Enable Vesktop";

    package = mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.vesktop;
    };

    arrpc.enable = mkOption {
      type = pkgs.lib.types.bool;
      default = true;
    };

    arrpc = {
      package = mkOption {
        type = pkgs.lib.types.package;
        default = pkgs.arrpc;
      };
    };

    settings = mkOption {
      type = json.type;
      default = { };
    };
    extraConfig = mkOption {
      type = json.type;
      default = { };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      cfg.package
      pkgs.arrpc
    ];
    xdg.configFile."vesktop/settings.json" = mkIf (cfg.settings != { }) {
      source = json.generate "settings.json" cfg.settings;
    };
    xdg.configFile."vesktop/settings/settings.json" = mkIf (cfg.extraConfig != { }) {
      source = json.generate "settings.json" cfg.extraConfig;
    };

    systemd.user.services.vesktop = {
      Install = {
        WantedBy = [
          (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
        ];
      };
      Unit = {
        Description = "A Custom Discord Client";
      };
      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        RestartSec = 5;
      };
    };

    # systemd.user.services.arRPC = mkIf cfg.arrpc.enable {
    #   Install = {
    #     WantedBy = [
    #       "graphical-session.target"
    #       "vesktop.service"
    #     ];
    #   };
    #   Unit = {
    #     Description = "Discord Rich Presence for browsers, and some custom clients";
    #   };
    #   Service = {
    #     ExecStart = lib.getExe cfg.arrpc.package;
    #     Restart = "on-failure";
    #     RestartSec = 5;
    #   };
    # };
  };
}
