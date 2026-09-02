{
  flake.modules.nixos.hyprland = {
    programs.hyprland.enable = true;
  };
  flake.modules.homeManager.hyprland =
    {
      pkgs,
      config,
      lib,
      ...
    }:
    let
      inherit (import ./_helpers.nix { inherit lib; }) mkEnv mkCurves mkAnimations;
    in
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
        plugins = [
          # pkgs.hyprland-scroll-overview
          # pkgs.hyprland-scroll-drag
        ];

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
              border_size = config.var.borderSize;
              col = {
                active_border = {
                  colors = [
                    "rgba(89b4faff)"
                    "rgba(a6e3a1ff)"
                  ];
                  angle = 125;
                };
                inactive_border = "rgba(1e1e2e00)";
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
                color = "rgba(00000030)";
                offset = "4, 4";
                range = 40;
                render_power = 3;
                scale = 1.0;
              };
            };

            debug.vfr = true;

            misc = {
              vrr = 1;
              enable_swallow = false;
              mouse_move_enables_dpms = true;
              key_press_enables_dpms = true;
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
              direct_scanout = 0;
              new_render_scheduling = false;
            };

            # plugin.scrolloverview = {
            #   scale = 0.5;
            #   workspace_gap = 20;
            #   layout = "vertical";
            # };

            # plugin.scrolldrag = {
            #   sensitivity = 1.0;
            #   deadzone = 8;
            #   workspace_switch_threshold = 300;
            # };

            dwindle = {
              force_split = 0;
              preserve_split = true;
              default_split_ratio = 1.0;
              special_scale_factor = 0.8;
              split_width_multiplier = 1.0;
              use_active_for_splits = true;
            };
            ecosystem.no_donation_nag = true;
            animations.enabled = true;
          };

          env = mkEnv {
            QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
            QT_QPA_PLATFORM = "wayland";
            NIXOS_OZONE_WL = "1";
          };

          # bezier.<name> = "x1, y1, x2, y2"
          # spring.<name> = "mass, stiffness, dampening"
          curve = mkCurves {
            bezier = {
              md3_decel = "0.05, 0.7, 0.1, 1";
              md3_accel = "0.3, 0, 0.8, 0.15";
              menu_decel = "0.1, 1, 0, 1";
              menu_accel = "0.38, 0.04, 1, 0.07";
            };
            spring = {
              spring_menu = "1, 240, 24";
              spring_window = "1, 240, 24";
              spring_open = "1, 240, 24";
              spring_workspace = "1, 240, 24";
              spring_special = "1, 240, 24";
            };
          };

          # "<leaf>, <speed>, <kind>:<curve>[, <style>]", "<leaf>, off" to disable.
          # Order matters: a parent leaf resets its children.
          animation = mkAnimations [
            "windows, 1, spring:spring_window"
            "windowsIn, 1, spring:spring_window, popin 40%"
            "windowsOut, 1, spring:spring_window, popin 40%"
            "border, off"
            "borderangle, off"
            "fade, off"
            "zoomFactor, 6, bezier:md3_decel"
            "layersIn, 3, spring:spring_menu, slide"
            "layersOut, 1.6, bezier:menu_accel, slide"
            "fadeLayersIn, 2, bezier:menu_decel"
            "fadeLayersOut, 1.6, bezier:menu_accel"
            "workspaces, 1, spring:spring_workspace, slidevert"
            "specialWorkspace, 1, spring:spring_special, slidefadevert 40%"
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
