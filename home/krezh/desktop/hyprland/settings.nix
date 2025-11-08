{ ... }:
{
  wayland.windowManager.hyprland.settings = {

    env = [
      "QT_WAYLAND_DISABLE_WINDOWDECORATION,1"
      "QT_QPA_PLATFORM=wayland"
    ];

    workspace = [
      "1,monitor:DP-1"
      "2,monitor:DP-1"
      "3,monitor:DP-1"
      "4,monitor:DP-2"
      "5,monitor:DP-2"
      "6,monitor:DP-2"
    ];

    xwayland = {
      force_zero_scaling = true;
    };

    cursor = {
      enable_hyprcursor = true;
      no_warps = true;
    };

    gestures = {
      workspace_swipe_cancel_ratio = 0.15;
    };

    input = {
      kb_layout = "se";
      follow_mouse = 2;
      accel_profile = "flat";
      numlock_by_default = true;
      touchpad = {
        natural_scroll = false;
      };
      sensitivity = "0.4"; # -1.0 - 1.0, 0 means no modification.
    };

    # plugin = {
    #   hyprview = {
    #     active_border_color = "$blue";
    #     inactive_border_color = "$base";
    #     bg_dim = "0.4";
    #     border_radius = 10;
    #     border_width = 2;
    #     margin = 10;
    #     gesture_distance = 200;
    #     workspace_indicator_enabled = 1;
    #     window_name_enabled = 1;
    #     window_name_font_size = 20;
    #     window_name_bg_opacity = "0.85";
    #     window_text_color = "0xFFFFFFFF";
    #   };
    # };

    general = {
      gaps_in = 5;
      gaps_out = 10;
      border_size = 2;
      "col.active_border" = "$blue";
      "col.inactive_border" = "$base";
      layout = "dwindle";

      allow_tearing = false;
      resize_on_border = true;
    };

    decoration = {
      rounding = 0;
      rounding_power = 4;

      # Global transparency settings
      active_opacity = 0.98;
      inactive_opacity = 0.98;

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
        # xray = false;
      };

      shadow = {
        enabled = true;
        color = "rgba(00000055)";
        ignore_window = true;
        offset = "0 15";
        range = 100;
        render_power = 3;
        scale = 0.97;
      };
    };

    animations = {
      enabled = true;
      bezier = [
        "easeOutQuint,0.23,1,0.32,1"
        "easeInOutCubic,0.65,0.05,0.36,1"
        "linear,0,0,1,1"
        "almostLinear,0.5,0.5,0.75,1.0"
        "quick,0.15,0,0.1,1"
        "specialWorkSwitch, 0.05, 0.7, 0.1, 1"
      ];

      animation = [
        "global, 1, 10, default"
        "border, 1, 5.39, easeOutQuint"
        "windows, 1, 4.79, easeOutQuint"
        "windowsIn, 1, 4.1, easeOutQuint, popin 87%"
        "windowsOut, 1, 1.49, linear, popin 87%"
        "fadeIn, 1, 1.73, almostLinear"
        "fadeOut, 1, 1.46, almostLinear"
        "fade, 1, 3.03, quick"
        "layers, 1, 3.81, easeOutQuint"
        "layersIn, 1, 4, easeOutQuint, fade"
        "layersOut, 1, 1.5, linear, fade"
        "fadeLayersIn, 1, 1.79, almostLinear"
        "fadeLayersOut, 1, 1.39, almostLinear"
        "workspaces, 1, 2, quick"
        "specialWorkspace, 1, 4, specialWorkSwitch, slidevert"
      ];
    };

    misc = {
      vfr = true;
      vrr = 2;
      enable_swallow = true;
      mouse_move_enables_dpms = true;
      key_press_enables_dpms = true;
      animate_manual_resizes = false;
      middle_click_paste = false;
      focus_on_activate = true;
      disable_hyprland_logo = true;
      disable_splash_rendering = true;
      disable_autoreload = true;
      session_lock_xray = true;
      new_window_takes_over_fullscreen = 2;
      render_unfocused_fps = 30;
    };

    dwindle = {
      pseudotile = true; # enable pseudotiling on dwindle
      force_split = 0;
      preserve_split = true;
      default_split_ratio = 1.0;
      special_scale_factor = 0.8;
      split_width_multiplier = 1.0;
      use_active_for_splits = true;
    };

    master = {
      mfact = 0.5;
      orientation = "left";
      special_scale_factor = 0.8;
      new_status = "master";
    };

    debug = {
      damage_tracking = 2; # leave it on 2 (full) unless you hate your GPU and want to make it suffer!
      # full_cm_proto = true;
    };
  };
}
