{
  flake.modules.nixos.hyprland = {
    programs.hyprland.enable = true;
  };
  flake.modules.homeManager.hyprland =
    { pkgs, config, ... }:
    {
      services.polkit-gnome.enable = true;
      catppuccin.hyprland.enable = false;
      wayland.windowManager.hyprland = {
        enable = true;
        xwayland.enable = true;
        configType = "lua";
        systemd = {
          enable = true;
          enableXdgAutostart = false;
          variables = [ "--all" ];
        };
        plugins = [ ];

        settings = {
          config = {
            xwayland.force_zero_scaling = true;

            cursor = {
              enable_hyprcursor = true;
              no_warps = true;
            };

            gestures.workspace_swipe_cancel_ratio = 0.15;

            input = {
              kb_layout = "se";
              kb_variant = "nodeadkeys";
              follow_mouse = 2;
              float_switch_override_focus = 0;
              accel_profile = "flat";
              numlock_by_default = true;
              touchpad.natural_scroll = false;
              sensitivity = 0.4;
            };

            general = {
              layout = "dwindle";
              gaps_in = 5;
              gaps_out = 10;
              border_size = 3;
              col = {
                active_border = {
                  colors = [
                    "rgba(89b4faff)"
                    "rgba(a6e3a1ff)"
                  ];
                  angle = 125;
                };
                inactive_border = "rgba(1e1e2eff)";
              };
            };

            decoration = {
              rounding = config.var.rounding;
              rounding_power = 4;
              active_opacity = config.var.opacity;
              inactive_opacity = config.var.opacity;
              blur = {
                enabled = false;
                passes = 4;
                size = 7;
                noise = 0.01;
                ignore_opacity = true;
                brightness = 1.0;
                contrast = 1.0;
                vibrancy = 0.8;
                vibrancy_darkness = 0.6;
                popups = true;
                popups_ignorealpha = 0.2;
              };
              shadow = {
                enabled = true;
                color = "rgba(1a1a1aaf)";
                offset = "0 40";
                range = 300;
                render_power = 4;
                scale = 0.90;
              };
            };

            debug.vfr = true;

            misc = {
              vrr = 2;
              enable_swallow = true;
              mouse_move_enables_dpms = true;
              key_press_enables_dpms = true;
              animate_manual_resizes = false;
              animate_mouse_windowdragging = false;
              middle_click_paste = false;
              focus_on_activate = true;
              disable_hyprland_logo = true;
              disable_splash_rendering = true;
              disable_autoreload = true;
              session_lock_xray = true;
              on_focus_under_fullscreen = 2;
              render_unfocused_fps = 60;
            };

            render = {
              direct_scanout = 1;
              new_render_scheduling = true;
            };

            dwindle = {
              force_split = 0;
              preserve_split = true;
              default_split_ratio = 1.0;
              special_scale_factor = 0.8;
              split_width_multiplier = 1.0;
              use_active_for_splits = true;
            };

            animations.enabled = true;
          };

          env = [
            {
              _args = [
                "QT_WAYLAND_DISABLE_WINDOWDECORATION"
                "1"
              ];
            }
            {
              _args = [
                "QT_QPA_PLATFORM"
                "wayland"
              ];
            }
            {
              _args = [
                "NIXOS_OZONE_WL"
                "1"
              ];
            }
          ];

          curve = [
            {
              _args = [
                "easeOutQuint"
                {
                  type = "bezier";
                  points = [
                    [
                      0.23
                      1
                    ]
                    [
                      0.32
                      1
                    ]
                  ];
                }
              ];
            }
            {
              _args = [
                "linear"
                {
                  type = "bezier";
                  points = [
                    [
                      0
                      0
                    ]
                    [
                      1
                      1
                    ]
                  ];
                }
              ];
            }
            {
              _args = [
                "almostLinear"
                {
                  type = "bezier";
                  points = [
                    [
                      0.5
                      0.5
                    ]
                    [
                      0.75
                      1.0
                    ]
                  ];
                }
              ];
            }
            {
              _args = [
                "quick"
                {
                  type = "bezier";
                  points = [
                    [
                      0.15
                      0
                    ]
                    [
                      0.1
                      1
                    ]
                  ];
                }
              ];
            }
            {
              _args = [
                "smooth"
                {
                  type = "bezier";
                  points = [
                    [
                      0.7
                      0.9
                    ]
                    [
                      0.1
                      1.0
                    ]
                  ];
                }
              ];
            }
          ];

          animation = [
            {
              leaf = "global";
              enabled = true;
              speed = 3.5;
              bezier = "smooth";
            }
            {
              leaf = "border";
              enabled = true;
              speed = 3.5;
              bezier = "smooth";
            }
            {
              leaf = "windows";
              enabled = true;
              speed = 3.5;
              bezier = "smooth";
            }
            {
              leaf = "fade";
              enabled = false;
            }
            {
              leaf = "layers";
              enabled = true;
              speed = 3.5;
              bezier = "smooth";
            }
            {
              leaf = "fadeLayers";
              enabled = true;
              speed = 3.5;
              bezier = "smooth";
            }
            {
              leaf = "workspaces";
              enabled = true;
              speed = 3.5;
              bezier = "smooth";
              style = "slidevert";
            }
            {
              leaf = "specialWorkspace";
              enabled = true;
              speed = 3.5;
              bezier = "smooth";
              style = "slidevert";
            }
          ];
        };
      };

      home.packages = [
        pkgs.tray-tui
        pkgs.hyprshade
        pkgs.hyprshutdown
      ];
    };
}
