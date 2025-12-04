{ pkgs, ... }:
{
  wayland.windowManager.hyprland.plugins = [
    pkgs.hyprlandPlugins.hyprscrolling
  ];

  wayland.windowManager.hyprland.settings = {
    general.layout = "scrolling";

    # https://github.com/hyprwm/hyprland-plugins/tree/main/hyprscrolling
    plugin.hyprscrolling = {
      fullscreen_on_one_column = true;
      column_width = 1;
      explicit_column_widths = "0.333, 0.5, 0.667, 1.0";
      focus_fit_method = 1;
      follow_focus = true;
    };

    bind = [
      # window control
      "$mainMod ALT, left, layoutmsg, movewindowto l"
      "$mainMod ALT, right, layoutmsg, movewindowto r"
      "$mainMod ALT, up, layoutmsg, movewindowto u"
      "$mainMod ALT, down, layoutmsg, movewindowto d"

      "$mainMod CTRL, left, layoutmsg, colresize -conf"
      "$mainMod CTRL, right, layoutmsg, colresize +conf"
      "$mainMod CTRL, up, resizeactive, 0 -80"
      "$mainMod CTRL, down, resizeactive, 0 80"
    ];
  };
}
