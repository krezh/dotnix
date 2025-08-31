{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hmModules.desktop.discord;
  inherit (lib) mkEnableOption mkOption mkIf;
in
{
  options.hmModules.desktop.discord = {
    enable = mkEnableOption "Enable discord";

    package = mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.discord;
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      cfg.package
    ];

    systemd.user.services.discord = {
      Install = {
        WantedBy = [
          (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
        ];
      };
      Unit = {
        Description = "A Discord Client";
      };
      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        RestartSec = 5;
      };
    };
  };
}
