{
  inputs,
  pkgs,
  config,
  ...
}:
let
  hyprlockConfig = config.programs.hyprlock.package;
  anyrunFlake = inputs.anyrun.packages.${pkgs.system};
  hyprkeysFlake = inputs.hyprkeys.packages.${pkgs.system}.hyprkeys;
in
{
  imports = [ ];

  wayland.windowManager.hyprland = {
    enable = true;
    catppuccin.enable = true;
    xwayland.enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd = {
      enable = true;
      variables = [ "--all" ]; # https://wiki.hyprland.org/Nix/Hyprland-on-Home-Manager/#programs-dont-work-in-systemd-services-but-do-on-the-terminal
    };

    #TODO: Split into separate files
    #TODO: Remove hypr.conf

    extraConfig = ''
      ${builtins.readFile ./hypr.conf}

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

      bind = [
        "$mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout}"
        "$mainMod,L,exec,${hyprlockConfig}/bin/hyprlock"
        "$mainMod,R,exec,${anyrunFlake.default}/bin/anyrun --plugins ${anyrunFlake.applications}/lib/libapplications.so"
        "$mainMod,K,exec,${hyprkeysFlake}/bin/hyprkeys -b -r | anyrun --plugins ${anyrunFlake.stdin}/lib/libstdin.so"
      ];

      general = {
        gaps_in = 3;
        gaps_out = 3;
        border_size = 2;
        "col.active_border" = "rgb(ff0000)";
        "col.inactive_border" = "rgb(00ff00)";
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
      inputs.hyprgrass.packages.${pkgs.system}.default
      # inputs.hyprland-plugins.packages.${pkgs.system}.hyprexpo
    ];
  };

  home.packages = with pkgs; [
    inputs.xdg-portal-hyprland.packages.${pkgs.system}.default
    catppuccin-cursors.mochaLight
  ];
}
