{
  inputs,
  pkgs,
  config,
  ...
}:
let
  hyprlockFlake = inputs.hyprlock.packages.${pkgs.system}.hyprlock;
  anyrunFlake = inputs.anyrun.packages.${pkgs.system};
  hyprkeysFlake = inputs.hyprkeys.packages.${pkgs.system}.hyprkeys;
in
{
  imports = [ ];
  wayland.windowManager.hyprland = {
    enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd.enable = true;
    xwayland.enable = false;

    extraConfig = ''
      ${builtins.readFile ./hypr.conf}

      # Generated from Nix
      bind = $mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout
      bind = $mainMod,L,exec,${hyprlockFlake}/bin/hyprlock
      bind = $mainMod,R,exec,${anyrunFlake.default}/bin/anyrun --plugins ${anyrunFlake.applications}/lib/libapplications.so
      bind = $mainMod,K,exec,${hyprkeysFlake}/bin/hyprkeys -b -r | anyrun --plugins ${anyrunFlake.stdin}/lib/libstdin.so
      # Generated from Nix
    '';
    settings = {
      monitor = [ ",preferred,auto,1" ];
      env = [
        "XCURSOR_SIZE,32"
        "XCURSOR_THEME,macOS-BigSur"
        "HYPRCURSOR_THEME,macOS-BigSur"
        "HYPRCURSOR_SIZE,32"
      ];

      cursor = {
        enable_hyprcursor = true;
      };

      general = {
        gaps_in = 2;
        gaps_out = 0;
        border_size = 0;
        # "col.active_border" = "${catppuccin_border}";
        # "col.inactive_border" = "${tokyonight_border}";
        layout = "dwindle";
        apply_sens_to_raw = 1; # whether to apply the sensitivity to raw input (e.g. used by games where you aim using your mouse)
      };

      decoration = {
        rounding = 0;
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
        # disable_splash_rendering = true;
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
      inputs.hyprgrass.packages.${pkgs.system}.default
      # inputs.hyprland-plugins.packages.${pkgs.system}.hyprexpo
    ];
  };

  programs.hyprlock = {
    enable = true;
    package = inputs.hyprlock.packages.${pkgs.system}.hyprlock;
    settings = {
      general = {
        disable_loading_bar = true;
        grace = 3; # in seconds
        hide_cursor = true;
        no_fade_in = false;
      };

      background = [
        {
          monitor = "";
          path = "screenshot";
          blur_passes = 3;
          blur_size = 8;
        }
      ];

      label = {
        monitor = "";
        text = "$TIME";
        text_align = "center"; # center/right or any value for default left. multi-line text alignment inside label container
        color = "rgba(200, 200, 200, 1.0)";
        font_size = 25;
        font_family = "Noto Sans";
        rotate = 0; # degrees, counter-clockwise
        position = "0, 80"; # x, y
        halign = "center";
        valign = "center";
      };

      input-field = [
        {
          size = "200, 50";
          position = "0, -80";
          monitor = "";
          dots_center = true;
          fade_on_empty = true;
          font_color = "rgb(202, 211, 245)";
          inner_color = "rgb(91, 96, 120)";
          outer_color = "rgb(24, 25, 38)";
          outline_thickness = 2;
          placeholder_text = ''<span foreground="##cad3f5"></span>'';
          shadow_passes = 2;
        }
      ];
    };
  };

  services.hypridle = {
    enable = true;
    package = inputs.hypridle.packages.${pkgs.system}.hypridle;
    settings = {
      general = {
        after_sleep_cmd = "hyprctl dispatch dpms on";
        ignore_dbus_inhibit = false;
        lock_cmd = "${hyprlockFlake}/bin/hyprlock";
      };
      listener = [
        {
          timeout = 5 * 60;
          on-timeout = "${hyprlockFlake}/bin/hyprlock";
        }
        {
          timeout = 10 * 60;
          on-timeout = "hyprctl dispatch dpms off";
          on-resume = "hyprctl dispatch dpms on";
        }
      ];
    };
  };

  home.packages = with pkgs; [ inputs.xdg-portal-hyprland.packages.${pkgs.system}.default ];
}
