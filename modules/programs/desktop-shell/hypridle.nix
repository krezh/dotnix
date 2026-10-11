{
  flake.modules.homeManager.desktop-shell =
    { config, lib, ... }:
    let
      hyprlock = lib.getExe config.programs.hyprlock.package;
      slumber = lib.getExe config.homeModules.slumber.package;
    in
    {
      services.hypridle = {
        enable = true;
        settings = {
          general = {
            after_sleep_cmd = "hyprctl dispatch dpms on";
            ignore_dbus_inhibit = false;
            lock_cmd = "${hyprlock}";
          };
          listener =
            (lib.optional config.homeModules.slumber.enable {
              timeout = 10 * 60;
              on-timeout = "${slumber}";
            })
            ++ [
              {
                timeout = 15 * 60;
                on-timeout = "${hyprlock}";
              }
              {
                timeout = 20 * 60;
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
