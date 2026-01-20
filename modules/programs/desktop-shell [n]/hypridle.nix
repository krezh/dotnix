{
  flake.modules.homeManager.desktop-shell = {
    config,
    lib,
    pkgs,
    ...
  }: let
    hyprlock = lib.getExe config.programs.hyprlock.package;
  in {
    services.hypridle = {
      enable = true;
      settings = {
        general = {
          after_sleep_cmd = "hyprctl dispatch dpms on";
          ignore_dbus_inhibit = false;
          lock_cmd = "${hyprlock}";
        };
        listener = [
          {
            timeout = 5 * 60;
            on-timeout = "${lib.getExe pkgs.hyprdvd} -s";
          }
          {
            timeout = 10 * 60;
            on-timeout = "${hyprlock}";
          }
          {
            timeout = 15 * 60;
            on-timeout = "hyprctl dispatch dpms off";
            on-resume = "hyprctl dispatch dpms on";
          }
          {
            timeout = 30 * 60;
            on-timeout = "systemctl suspend";
          }
        ];
      };
    };
  };
}
