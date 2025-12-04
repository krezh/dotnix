{ var, ... }:
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
      kb_variant = "nodeadkeys";
      follow_mouse = 2;
      float_switch_override_focus = 0;
      accel_profile = "flat";
      numlock_by_default = true;
      touchpad = {
        natural_scroll = false;
      };
      sensitivity = "0.4"; # -1.0 - 1.0, 0 means no modification.
    };

    plugin = {
      hyprexpo = {
        columns = 2;
        gap_size = 5;
        bg_col = "rgb(111111)";
        workspace_method = "center current"; # [center/first] [workspace] e.g. first 1 or center m+1
        gesture_distance = 300; # how far is the "max" for the gesture
        skip_empty = true;
      };
    };

    general = {
      layout = "dwindle";
      gaps_in = 5;
      gaps_out = 10;
      border_size = 3;
      "col.active_border" = "$blue $green 125deg";
      "col.inactive_border" = "$base";
      allow_tearing = false;
    };

    decoration = {
      rounding = var.rounding;
      rounding_power = 4;

      # Global transparency settings
      active_opacity = var.opacity;
      inactive_opacity = var.opacity;

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
      force_split = 0;
      preserve_split = true;
      default_split_ratio = 1.0;
      special_scale_factor = 0.8;
      split_width_multiplier = 1.0;
      use_active_for_splits = true;
    };

    debug = {
      damage_tracking = 2; # leave it on 2 (full) unless you hate your GPU and want to make it suffer!
      # full_cm_proto = true;
    };
  };
}
