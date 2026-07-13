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
              font_family = config.var.fonts.sans;
              panel_overlap = 0;
              start = [
                "cpu"
                "temp"
                "ram"
                "disk-nix"
                "disk-home"
                "active_window"
                "media"
              ];
              center = [ "workspaces" ];
              end = [
                "audio_visualizer"
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
          control_center = {
            sidebar = "full";
            sidebar_section = "full";
            calendar = {
              show_events_card = false;
            };
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
            widget_order = [
              "lockscreen-login-box@DP-2"
              "lockscreen-login-box@DP-1"
            ];
            grid = {
              cell_size = 16;
              major_interval = 4;
              visible = true;
            };
            widget = {
              "lockscreen-login-box@DP-1" = {
                box_height = 0.0;
                box_width = 0.0;
                cx = 1280.0;
                cy = 1317.0;
                output = "DP-1";
                rotation = 0.0;
                type = "login_box";
              };
              "lockscreen-login-box@DP-2" = {
                box_height = 0.0;
                box_width = 0.0;
                cx = 1280.0;
                cy = 1317.0;
                output = "DP-2";
                rotation = 0.0;
                type = "login_box";
              };
            };
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
            position_vertical = "center_right";
            kinds = {
              media = false;
            };
          };
          plugins = {
            source = [
              {
                auto_update = true;
                kind = "git";
                location = "https://github.com/noctalia-dev/official-plugins";
                name = "official";
              }
              {
                auto_update = true;
                kind = "git";
                location = "https://github.com/noctalia-dev/community-plugins";
                name = "community";
              }
            ];
          };
          shell = {
            avatar_path = "${config.home.homeDirectory}/.face";
            clipboard_enabled = true;
            font_family = config.var.fonts.sans;
            launch_apps_as_systemd_services = true;
            offline_mode = true;
            screen_time_enabled = true;
            settings_show_advanced = true;
            telemetry_enabled = true;
            launcher = {
              categories = false;
            };
            panel = {
              borders = false;
              open_near_click_control_center = true;
            };
          };
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
              color_2 = "tertiary";
              mirrored = false;
            };
            clock = {
              anchor = true;
              font_weight = 700;
            };
            cpu = {
              display = "text";
            };
            ram = {
              display = "text";
            };
            temp = {
              display = "text";
            };
            media = {
              hide_when_no_media = true;
            };
            network = {
              show_label = false;
            };
            disk-nix = {
              type = "sysmon";
              display = "text";
              stat = "disk_pct";
              path = "/nix";
            };
            disk-home = {
              type = "sysmon";
              display = "text";
              stat = "disk_pct";
              path = "/home";
            };
          };
        };
      };
    };
}
