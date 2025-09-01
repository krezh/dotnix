{

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
  };

  gestures = {
    workspace_swipe = true;
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

  plugins = { };

  general = {
    gaps_in = 3;
    gaps_out = 3;
    border_size = 1;
    "col.active_border" = "$blue";
    "col.inactive_border" = "$base";
    layout = "dwindle";
  };

  decoration = {
    rounding = 15;
    blur = {
      enabled = true;
      size = 4;
      passes = 2;
      new_optimizations = true;
      ignore_opacity = true;
      noise = 1.17e-2;
      contrast = 1.3;
      brightness = 1;
      xray = true;
    };
  };

  animations = {
    enabled = true;
    bezier = [
      "wind, 0.05, 0.9, 0.1, 1.05"
      "winIn, 0.1, 1.1, 0.1, 1.1"
      "winOut, 0.3, -0.3, 0, 1"
      "liner, 1, 1, 1, 1"
    ];
    animation = [
      "windows, 1, 6, wind, slide"
      "windowsIn, 1, 6, winIn, slide"
      "windowsOut, 1, 5, winOut, slide"
      "windowsMove, 1, 5, wind, slide"
      "border, 1, 1, liner"
      "borderangle, 1, 30, liner, loop"
      "fade, 1, 10, default"
      "workspaces, 1, 5, wind"
    ];
  };

  misc = {
    vfr = true; # misc:no_vfr -> misc:vfr. bool, heavily recommended to leave at default on. Saves on CPU usage.
    vrr = 2; # misc:vrr -> Adaptive sync of your monitor. 0 (off), 1 (on), 2 (fullscreen only). Default 0 to avoid white flashes on select hardware.
    disable_hyprland_logo = true;
    disable_splash_rendering = true;
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
  };
}
