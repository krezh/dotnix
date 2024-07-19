{
  inputs,
  pkgs,
  config,
  lib,
  ...
}:
let
  hyprlockConfig = config.programs.hyprlock.package;
  anyrunFlake = inputs.anyrun.packages.${pkgs.system};
  anyrun = {
    stdin = "${anyrunFlake.stdin}/lib/libstdin.so";
    applications = "${anyrunFlake.applications}/lib/libapplications.so";
    bin = "${anyrunFlake.default}/bin/anyrun";
  };
  hyprkeysFlake = inputs.hyprkeys.packages.${pkgs.system}.hyprkeys;
  weztermConfig = config.programs.wezterm.package;
in
{
  imports = [ ];

  wayland.windowManager.hyprland = {
    enable = true;
    catppuccin.enable = true;
    xwayland.enable = false;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd = {
      enable = true;
      variables = [ "--all" ]; # https://wiki.hyprland.org/Nix/Hyprland-on-Home-Manager/#programs-dont-work-in-systemd-services-but-do-on-the-terminal
    };

    #TODO: Split into separate files

    extraConfig = ''
      # Generated from Nix
      # Generated from Nix
    '';

    settings = {
      monitor = [ ",preferred,auto,1" ];
      "$mainMod" = "SUPER";
      "$SupShft" = "SUPER SHIFT";

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

      bind = [
        # Hyprland binds
        "$mainMod,Q,killactive"
        "$mainMod,V,togglefloating"
        "$mainMod,P,pseudo"
        "$mainMod,J,togglesplit"
        "$mainMod,F,fullscreen,1"
        "$SupShft,F,fullscreen,2"
        "$SupShft,LEFT,movewindow,l"
        "$SupShft,RIGHT,movewindow,r"
        "$SupShft,UP,movewindow,u"
        "$SupShft,DOWN,movewindow,d"

        # Move focus with mainMod + arrow keys
        "$mainMod,left,movefocus,l"
        "$mainMod,right,movefocus,r"
        "$mainMod,up,movefocus,u"
        "$mainMod,down,movefocus,d"

        # Switch workspaces with mainMod + [0-9]
        "$mainMod,1,workspace,1"
        "$mainMod,2,workspace,2"
        "$mainMod,3,workspace,3"
        "$mainMod,4,workspace,4"
        "$mainMod,5,workspace,5"
        "$mainMod,6,workspace,6"
        "$mainMod,7,workspace,7"
        "$mainMod,8,workspace,8"
        "$mainMod,9,workspace,9"
        "$mainMod,0,workspace,10"

        # Scratchpad
        "$mainMod,S,togglespecialworkspace"
        "$mainModSHIFT,S,movetoworkspace,special"

        # Move active window to a workspace with mainMod + SHIFT + [0-9]
        "$mainModSHIFT,1,movetoworkspace,1"
        "$mainModSHIFT,2,movetoworkspace,2"
        "$mainModSHIFT,3,movetoworkspace,3"
        "$mainModSHIFT,4,movetoworkspace,4"
        "$mainModSHIFT,5,movetoworkspace,5"
        "$mainModSHIFT,6,movetoworkspace,6"
        "$mainModSHIFT,7,movetoworkspace,7"
        "$mainModSHIFT,8,movetoworkspace,8"
        "$mainModSHIFT,9,movetoworkspace,9"
        "$mainModSHIFT,0,movetoworkspace,10"

        # Scroll through existing workspaces with mainMod + scroll
        "$mainMod,mouse_down,workspace,e+1"
        "$mainMod,mouse_up,workspace,e-1"

        "$mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout"
        "$mainMod,L,exec,${hyprlockConfig}/bin/hyprlock"
        "$mainMod,R,exec,${anyrunFlake.default}/bin/anyrun --plugins ${anyrun.applications}"
        "$mainMod,K,exec,${hyprkeysFlake}/bin/hyprkeys -b -r | anyrun --plugins ${anyrun.stdin}"
        # Applications
        "$mainMod,B,exec,${pkgs.firefox}/bin/firefox"
        "$mainMod,E,exec,${pkgs.cinnamon.nemo}/bin/nemo"
        "$mainMod,RETURN,exec,${weztermConfig}/bin/wezterm"
        "$SupShft,RETURN,exec,${weztermConfig}/bin/wezterm"
        "$mainMod,C,exec,${pkgs.clipman}/bin/clipman pick -t rofi"
        "$mainMod,O,exec,${pkgs.obsidian}/bin/obsidian"
        # Hyprland Plugins
        # "$mainMod,TAB,hyprexpo:expo,toggle" # can be: toggle, off/disable or on/enable
      ];

      bindm = [
        # Move/resize windows with mainMod + SHIFT and dragging
        "$mainMod,       mouse:272, movewindow"
        "$mainMod SHIFT, mouse:272, resizewindow"
      ];

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
