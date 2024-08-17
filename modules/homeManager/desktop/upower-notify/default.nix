{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.upower-notify;
in
{
  options.hmModules.desktop.upower-notify = {
    enable = lib.mkEnableOption "upower-notify";
  };

  config = lib.mkIf cfg.enable {

    systemd.user.services.upower-notify = {
      Unit = {
        Description = "";
        Documentation = "";
      };
      Service = {
        PassEnvironment = [ "PATH" ];
        ExecStart = "${pkgs.upower-notify}/bin/upower-notify";
        Restart = "on-failure";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
