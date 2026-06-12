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
          audio = {
            enable_overdrive = false;
            enable_sounds = false;
          };
          bar = {
            main = {
              background_opacity = 0.9;
              capsule = false;
              contact_shadow = true;
              start = [
                "cpu"
                "temp"
                "ram"
                "media"
                "audio_visualizer"
                "active_window"
              ];
              center = [ "workspaces" ];
              end = [
                "tray"
                "network"
                "bluetooth"
                "volume"
                "brightness"
                "battery"
                "clock"
                "notifications"
                "session"
              ];
              margin_ends = 10;
              margin_h = 10;
              margin_v = 5;
              padding = 10;
              position = "top";
              radius = 15;
              thickness = 30;
              widget_spacing = 10;
            };
          };
          brightness = {
            enable_ddcutil = false;
          };
          config = { };
          control_center = {
            sidebar = "full";
            sidebar_section = "full";
          };
          dock = {
            active_monitor_only = true;
            auto_hide = true;
            show_dots = true;
          };
          location = {
            address = "Sweden, Bålsta";
          };
          lockscreen_widgets = {
            enabled = false;
            schema_version = 2;
            grid = {
              cell_size = 16;
              major_interval = 4;
              visible = true;
            };
            widget = { };
          };
          nightlight = {
            enabled = false;
          };
          notification = {
            background_opacity = 0.98;
            layer = "overlay";
          };
          osd = {
            background_opacity = 0.98;
            orientation = "vertical";
            position = "center_right";
          };
          shell = {
            avatar_path = "${config.home.homeDirectory}/.face";
            clipboard_enabled = true;
            font_family = "${config.var.fonts.sans}";
            launch_apps_as_systemd_services = true;
            offline_mode = true;
            screen_time_enabled = true;
            telemetry_enabled = false;
            panel = {
              borders = false;
              open_near_click_control_center = true;
              launcher_categories = false;
            };
          };
          templates = { };
          theme = {
            builtin = "Catppuccin";
            community_palette = "Catppuccin Lavender";
            mode = "dark";
            source = "builtin";
          };
          wallpaper = {
            enabled = false;
          };
          weather = {
            enabled = true;
            unit = "celsius";
          };
          widget = {
            workspaces = {
              display = "none";
              focused_color = "error";
            };
            audio_visualizer = {
              high_color = "tertiary";
              mirrored = false;
            };
            cpu = {
              display = "graph";
            };
            network = {
              show_label = false;
            };
            ram = {
              display = "graph";
            };
            sysmon = {
              display = "graph";
              stat = "cpu_temp";
            };
            temp = {
              display = "graph";
            };
          };
        };
      };
    };
}
