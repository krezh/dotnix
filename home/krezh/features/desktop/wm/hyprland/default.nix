{ inputs, pkgs, ... }:
{
  imports = [ ./binds.nix ];

  wayland.windowManager.hyprland = {
    enable = true;
    catppuccin.enable = true;
    xwayland.enable = false;
    #package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd = {
      enable = true;
      variables = [ "--all" ]; # https://wiki.hyprland.org/Nix/Hyprland-on-Home-Manager/#programs-dont-work-in-systemd-services-but-do-on-the-terminal
    };

    extraConfig = '''';

    settings = {
      monitor = [ ",preferred,auto,1" ];

      env = [ "QT_WAYLAND_DISABLE_WINDOWDECORATION,1" ];

      cursor = {
        enable_hyprcursor = true;
      };

      misc = {
        enable_swallow = true;
        mouse_move_enables_dpms = true;
        animate_manual_resizes = false;
      };

      gestures = {
        workspace_swipe = true;
        workspace_swipe_cancel_ratio = 0.15;
      };

      input = {
        kb_layout = "se";
        follow_mouse = 1;
        touchpad = {
          natural_scroll = false;
        };
        sensitivity = "0.4"; # -1.0 - 1.0, 0 means no modification.
      };

      plugins = {
        hyprfocus = {
          enabled = true;
          animate_floating = true;
          animate_workspacechange = true;
          focus_animation = "shrink";
          bezier = [
            "bezIn, 0.5,0.0,1.0,0.5"
            "bezOut, 0.0,0.5,0.5,1.0"
            "overshot, 0.05, 0.9, 0.1, 1.05"
            "smoothOut, 0.36, 0, 0.66, -0.56"
            "smoothIn, 0.25, 1, 0.5, 1"
            "realsmooth, 0.28,0.29,.69,1.08"
          ];
          flash = {
            flash_opacity = 0.95;
            in_bezier = "realsmooth";
            in_speed = 0.5;
            out_bezier = "realsmooth";
            out_speed = 3;
          };
          shrink = {
            shrink_percentage = 0.99;
            in_bezier = "realsmooth";
            in_speed = 1;
            out_bezier = "realsmooth";
            out_speed = 2;
          };
        };

        # hyprexpo = {
        #   columns = 3;
        #   gap_size = 5;
        #   bg_col = "$base";
        #   workspace_method = "center current";
        #   enable_gesture = true;
        #   gesture_fingers = 3;
        #   gesture_distance = 300;
        #   gesture_positive = true;
        # };
      };

      general = {
        gaps_in = 3;
        gaps_out = 3;
        border_size = 1;
        "col.active_border" = "$blue";
        "col.inactive_border" = "$base";
        layout = "dwindle";
        apply_sens_to_raw = 1; # whether to apply the sensitivity to raw input (e.g. used by games where you aim using your mouse)
      };

      decoration = {
        rounding = 15;
        shadow_ignore_window = true;
        drop_shadow = false;
        shadow_range = 20;
        shadow_render_power = 3;
        # "col.shadow" = "rgb(${oxocarbon_background})";
        # "col.shadow_inactive" = "${background}";
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
        bezier = [ "1, 0.23, 1, 0.32, 1" ];
        animation = [
          "windows, 1, 5, 1"
          "windowsIn, 1, 5, 1, slide"
          "windowsOut, 1, 5, 1, slide"
          "border, 1, 5, default"
          "borderangle, 1, 5, default"
          "fade, 1, 5, default"
          "workspaces, 1, 5, 1, slidefade 30%"
        ];
      };

      misc = {
        vfr = true; # misc:no_vfr -> misc:vfr. bool, heavily recommended to leave at default on. Saves on CPU usage.
        vrr = false; # misc:vrr -> Adaptive sync of your monitor. 0 (off), 1 (on), 2 (fullscreen only). Default 0 to avoid white flashes on select hardware.
        disable_hyprland_logo = true;
        disable_splash_rendering = true;
      };

      dwindle = {
        pseudotile = true; # enable pseudotiling on dwindle
        force_split = 0;
        preserve_split = true;
        default_split_ratio = 1.0;
        no_gaps_when_only = false;
        special_scale_factor = 0.8;
        split_width_multiplier = 1.0;
        use_active_for_splits = true;
      };

      master = {
        mfact = 0.5;
        orientation = "right";
        special_scale_factor = 0.8;
        new_status = "slave";
        no_gaps_when_only = false;
      };

      debug = {
        damage_tracking = 2; # leave it on 2 (full) unless you hate your GPU and want to make it suffer!
      };

    };
    plugins = [
      inputs.hyprfocus.packages.${pkgs.system}.hyprfocus
      # inputs.hyprhook.packages.${pkgs.system}.hyprhook
      # inputs.hyprland-plugins.packages.${pkgs.system}.hyprexpo
    ];
  };

  home.packages = with pkgs; [
    inputs.xdg-portal-hyprland.packages.${pkgs.system}.default
    clipman
    flameshot
    brightnessctl
    light
  ];
}
