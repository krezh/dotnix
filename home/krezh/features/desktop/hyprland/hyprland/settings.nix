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
    };

    misc = {
      enable_swallow = true;
      mouse_move_enables_dpms = true;
      key_press_enables_dpms = true;
      animate_manual_resizes = false;
      middle_click_paste = false;
      focus_on_activate = true;
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

    plugin = {
      hyprview = {
        active_border_color = "$blue";
        inactive_border_color = "$base";
        bg_dim = "0.4";
        border_radius = 10;
        border_width = 2;
        margin = 10;
        gesture_distance = 200;
        workspace_indicator_enabled = 1;
        window_name_enabled = 1;
        window_name_font_size = 20;
        window_name_bg_opacity = "0.85";
        window_text_color = "0xFFFFFFFF";
      };
    };

    general = {
      gaps_in = 5;
      gaps_out = 5;
      border_size = 2;
      "col.active_border" = "$blue";
      "col.inactive_border" = "$base";
      layout = "dwindle";

      allow_tearing = false;
      resize_on_border = true;
    };

    decoration = {
      rounding = 10;
      rounding_power = 3;

      # Global transparency settings
      active_opacity = 0.85;
      inactive_opacity = 0.85;

      #screen_shader = "${./brightness.glsl}";

      blur = {
        enabled = true;
        passes = 4;
        size = 7;
        noise = 0.01;
        ignore_opacity = true;
        # brightness = 1.0;
        # contrast = 1.0;
        # vibrancy = 0.8;
        # vibrancy_darkness = 0.6;
        # popups = true;
        # popups_ignorealpha = 0.2;
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
        "specialWorkSwitch, 0.05, 0.7, 0.1, 1"
        "emphasizedAccel, 0.3, 0, 0.8, 0.15"
        "emphasizedDecel, 0.05, 0.7, 0.1, 1"
        "standard, 0.2, 0, 0, 1"
      ];
      animation = [
        "layersIn, 1, 5, emphasizedDecel, slide"
        "layersOut, 1, 4, emphasizedAccel, slide"
        "fadeLayers, 1, 5, standard"
        "windowsIn, 1, 5, emphasizedDecel"
        "windowsOut, 1, 3, emphasizedAccel"
        "windowsMove, 1, 6, standard"
        "workspaces, 1, 5, standard"
        "specialWorkspace, 1, 4, specialWorkSwitch, slidefadevert 15%"
        "fade, 1, 6, standard"
        "fadeDim, 1, 6, standard"
        "border, 1, 6, standard"
      ];
    };

    misc = {
      vfr = true; # misc:no_vfr -> misc:vfr. bool, heavily recommended to leave at default on. Saves on CPU usage.
      vrr = 2; # misc:vrr -> Adaptive sync of your monitor. 0 (off), 1 (on), 2 (fullscreen only). Default 0 to avoid white flashes on select hardware.
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
      orientation = "right";
      special_scale_factor = 0.8;
      new_status = "slave";
    };

    debug = {
      damage_tracking = 2; # leave it on 2 (full) unless you hate your GPU and want to make it suffer!
      # full_cm_proto = true;
    };
  };
}
