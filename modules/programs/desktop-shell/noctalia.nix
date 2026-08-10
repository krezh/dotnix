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
          location = { };
          lockscreen_widgets = {
            enabled = false;
          };
          nightlight = {
            enabled = false;
          };
          notification = {
            background_opacity = config.var.opacity;
            layer = "overlay";
          };
          osd = {
            background_opacity = config.var.opacity;
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
                kind = "git";
                location = "https://github.com/noctalia-dev/official-plugins";
                name = "official";
              }
              {
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
            session = {
              actions = [
                {
                  enabled = true;
                  action = "lock";
                  countdown_seconds = 0;
                  shortcut = 1;
                  variant = "default";
                }
                {
                  enabled = true;
                  action = "logout";
                  countdown_seconds = 0;
                  shortcut = 2;
                  variant = "default";
                }
                {
                  enabled = true;
                  action = "lock_and_suspend";
                  countdown_seconds = 0;
                  shortcut = 3;
                  variant = "default";
                }
                {
                  enabled = true;
                  action = "reboot";
                  command = "systemd-run --user --scope --collect -- hyprshutdown -t 'Restarting...' --post-cmd 'reboot'";
                  countdown_seconds = 0;
                  shortcut = 4;
                  variant = "default";
                }
                {
                  action = "shutdown";
                  command = "systemd-run --user --scope --collect -- hyprshutdown -t 'Shutting down...' --post-cmd 'shutdown -P 0'";
                  countdown_seconds = 0;
                  enabled = true;
                  shortcut = 5;
                  variant = "destructive";
                }
              ];
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
              focused_color = "error";
              show_label = false;
              show_labels = false;
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
              visualization = "none";
            };
            ram = {
              visualization = "none";
            };
            temp = {
              visualization = "none";
            };
            media = {
              hide_when_no_media = true;
            };
            network = {
              show_label = false;
            };
            disk-nix = {
              type = "sysmon";
              visualization = "none";
              stat = "disk_used_pct";
              path = "/nix";
            };
            disk-home = {
              type = "sysmon";
              visualization = "none";
              stat = "disk_used_pct";
              path = "/home";
            };
          };
        };
      };
    };
}
