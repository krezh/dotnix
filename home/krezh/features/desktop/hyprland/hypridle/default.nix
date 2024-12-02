{
  inputs,
  pkgs,
  config,
  ...
}:
{
  services.hypridle = {
    enable = true;
    package = inputs.hypridle.packages.${pkgs.system}.hypridle;
    settings = {
      general = {
        after_sleep_cmd = "hyprctl dispatch dpms on";
        ignore_dbus_inhibit = false;
        lock_cmd = "${config.programs.hyprlock.package}/bin/hyprlock";
      };
      listener = [
        {
          timeout = 5 * 60;
          on-timeout = "${config.programs.hyprlock.package}/bin/hyprlock";
        }
        {
          timeout = 10 * 60;
          on-timeout = "hyprctl dispatch dpms off";
          on-resume = "hyprctl dispatch dpms on";
        }
      ];
    };
  };
}
