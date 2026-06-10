{ inputs, ... }:
{
  flake.modules.homeManager.desktop-shell =
    { config, ... }:
    {
      imports = [ inputs.noctalia.homeModules.default ];

      programs.noctalia = {
        enable = true;
        systemd.enable = true;

        settings = {
          shell = {
            font_family = config.var.fonts.sans;
            telemetry_enabled = false;
            avatar_path = "/home/${config.var.username}/.face";
          };

          theme = {
            mode = "dark";
            source = "builtin";
            builtin = "Tokyo-Night";
          };

          location.address = "Sweden, Bålsta";

          weather = {
            enabled = true;
            unit = "celsius";
          };

          wallpaper.enabled = false;

          notification.background_opacity = 0.98;

          osd.background_opacity = 1.0;

          audio = {
            enable_overdrive = false;
            enable_sounds = false;
          };

          brightness.enable_ddcutil = false;

          nightlight.enabled = false;

          bar.main = {
            position = "top";
            background_opacity = 0.9;
            radius = 12;
            margin_h = 10;
            margin_v = 5;
            padding = 2;
            widget_spacing = 6;
            start = [ "workspaces" ];
            center = [ "clock" ];
            end = [
              "media"
              "tray"
              "network"
              "bluetooth"
              "volume"
              "brightness"
              "battery"
              "control-center"
              "notifications"
              "session"
            ];
          };
        };
      };
    };
}
