{ ... }:
{
  wayland.windowManager.hyprland.settings.workspace = [
    "w[tv1]s[false], gapsout:0, gapsin:0"
    "f[1]s[false], gapsout:0, gapsin:0"
  ];
  wayland.windowManager.hyprland.settings.windowrule = [
    "bordersize 0, floating:0, onworkspace:w[tv1]s[false]"
    "rounding 0, floating:0, onworkspace:w[tv1]s[false]"
    "bordersize 0, floating:0, onworkspace:f[1]s[false]"
    "rounding 0, floating:0, onworkspace:f[1]s[false]"
  ];
}
