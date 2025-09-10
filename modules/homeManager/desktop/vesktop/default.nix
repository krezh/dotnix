{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hmModules.desktop.vesktop;
  inherit (lib) mkEnableOption mkOption mkIf;
in
{
  options.hmModules.desktop.vesktop = {
    enable = mkEnableOption "Enable Vesktop";

    package = mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.vesktop;
    };

    settings = mkOption {
      type = lib.types.attrs;
      default = { };
    };

    vencord.settings = mkOption {
      type = lib.types.attrs;
      default = { };
    };
  };

  config = mkIf cfg.enable {

    programs.vesktop.enable = true;
    programs.vesktop.package = cfg.package;
    programs.vesktop.settings = cfg.settings;
    programs.vesktop.vencord.settings = cfg.vencord.settings;
    programs.vesktop.vencord.useSystem = true;

    services.arrpc.enable = true;

    # xdg.configFile."vesktop/settings.json" = mkIf (cfg.settings != { }) {
    # source = json.generate "settings.json" cfg.settings;
    # };
    # xdg.configFile."vesktop/settings/settings.json" = mkIf (cfg.extraConfig != { }) {
    # source = json.generate "settings.json" cfg.extraConfig;
    # };

    # systemd.user.services.vesktop = {
    #   Install = {
    #     WantedBy = [
    #       (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
    #     ];
    #   };
    #   Unit = {
    #     Description = "A Custom Discord Client";
    #   };
    #   Service = {
    #     ExecStart = lib.getExe cfg.package;
    #     Restart = "on-failure";
    #     RestartSec = 5;
    #   };
    # };

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
