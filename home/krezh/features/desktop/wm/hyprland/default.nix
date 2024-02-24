{ inputs, pkgs, ... }:
{
  imports = [
    inputs.hyprlock.homeManagerModules.hyprlock
    inputs.ags.homeManagerModules.default
  ];
  wayland.windowManager.hyprland = {
    enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd.enable = true;
    xwayland.enable = true;
    extraConfig = ''
      ${builtins.readFile ./hypr.conf}
    '';
    settings = {
      "$mod" = "SUPER";
      exec-once = [      
      ];
      input = {
        kb_layout = "se";
        follow_mouse = 1;
        accel_profile = "flat";
        sensitivity = 0;
        touchpad = {
          natural_scroll = "false";
        };
      };
      monitor = [
        ",preferred,auto,1"
      ];
      bind = [
        "$mod,        RETURN, exec, wezterm"
        "$mod, 	      L,      exec, hyprlock"
        "$mod,        Q,      killactive"
        "$mod,        V,      togglefloating"
        "$mod SHIFT,  LEFT,    movewindow, l"
        "$mod SHIFT,  RIGHT,   movewindow, r"
        "$mod SHIFT,  UP,      movewindow, u"
        "$mod SHIFT,  RIGHT,   movewindow, d"
      ];
    };
  };

  programs.hyprlock = {
    enable = true;
  };
}
