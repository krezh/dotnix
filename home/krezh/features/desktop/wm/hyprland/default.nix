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
      env = [
        "XCURSOR_SIZE,32"
        "XCURSOR_THEME,macOS-BigSur"
        "HYPRCURSOR_THEME,macOS-BigSur"
        "HYPRCURSOR_SIZE,32"
      ];
      cursor = {
        enable_hyprcursor = true;
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
          timeout = 900;
          on-timeout = "${hyprlockFlake}/bin/hyprlock";
        }
        {
          timeout = 1200;
          on-timeout = "hyprctl dispatch dpms off";
          on-resume = "hyprctl dispatch dpms on";
        }
      ];
    };
  };

  home.packages = with pkgs; [ inputs.xdg-portal-hyprland.packages.${pkgs.system}.default ];
}
