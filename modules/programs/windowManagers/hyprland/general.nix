{
  flake.modules.nixos.hyprland = {
    programs.hyprland.enable = true;
  };
  flake.modules.homeManager.hyprland =
    {
      pkgs,
      config,
      ...
    }:
    {
      services.polkit-gnome.enable = true;
      xdg.portal = {
        enable = true;
        extraPortals = with pkgs; [
          xdg-desktop-portal-hyprland
          xdg-desktop-portal-gtk
        ];
        # xdgOpenUsePortal = true;
        configPackages = [ config.wayland.windowManager.hyprland.package ];
        config.hyprland = {
          default = [
            "hyprland"
            "gtk"
          ];
          "org.freedesktop.impl.portal.FileChooser" = "gtk";
          "org.freedesktop.impl.portal.Print" = "gtk";
        };
      };
      wayland.windowManager.hyprland = {
        enable = true;
        xwayland.enable = true;
        systemd = {
          enable = true;
          enableXdgAutostart = true;
          variables = [ "--all" ];
        };
        plugins = [
          # pkgs.hyprlandPlugins.hyprexpo
        ];

        settings = {
          env = [
            "QT_WAYLAND_DISABLE_WINDOWDECORATION,1"
            "QT_QPA_PLATFORM=wayland"
          ];

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
            sensitivity = "0.4";
          };

          # plugin.hyprexpo = {
          #   columns = 2;
          #   gap_size = 5;
          #   bg_col = "rgb(111111)";
          #   workspace_method = "center current";
          #   gesture_distance = 300;
          #   skip_empty = true;
          # };

          general = {
            layout = "dwindle";
            gaps_in = 5;
            gaps_out = 10;
            border_size = 3;
            "col.active_border" = "$blue $green 125deg";
            "col.inactive_border" = "$base";
          };

          decoration = {
            inherit (config.var) rounding;
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
              ignore_window = true;
              offset = "0 40";
              range = 300;
              render_power = 4;
              scale = 0.90;
            };
          };

          misc = {
            vfr = true;
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

          animations = {
            enabled = true;
            bezier = [
              "easeOutQuint,0.23,1,0.32,1"
              "linear,0,0,1,1"
              "almostLinear,0.5,0.5,0.75,1.0"
              "quick,0.15,0,0.1,1"
              "smooth,0.7,0.9,0.1,1.0"
            ];
            animation = [
              "global, 1, 3.5, smooth"
              "border, 1, 3.5, smooth"
              "windows, 1, 3.5, smooth"
              "fade, 0"
              "layers, 1, 3.5, smooth"
              "fadeLayers, 1, 3.5, smooth"
              "workspaces, 1, 3.5, smooth, slidevert"
              "specialWorkspace, 1, 3.5, smooth, slidevert"
            ];
          };
        };
      };

      # XDPH config
      xdg.configFile."hypr/xdph.conf".text = ''
        screencopy {
          max_fps = 120
          allow_token_by_default = true
        }
      '';
      home.packages = [
        pkgs.tray-tui
        pkgs.hyprpicker
        pkgs.hyprshade
        pkgs.hyprshutdown
      ];
    };
}
