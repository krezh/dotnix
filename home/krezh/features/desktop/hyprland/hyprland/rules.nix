let
  # Simple helper to create opacity rules for a list of app classes
  opaque = apps: map (app: "opacity 1.0 override,class:^(${app})$") apps;
  opacity = value: apps: map (app: "opacity ${toString value} override,class:^(${app})$") apps;
in
{
  wayland.windowManager.hyprland = {
    settings = {
      layerrule = [
        "blur,rofi"
        # Fuzzel
        "animation popin 80%, launcher"
        "blur, launcher"
        "animation fade, hyprpicker" # Colour picker out animation
        "animation fade, logout_dialog" # wlogout
        "animation fade, selection" # slurp
        "animation fade, wayfreeze"
      ];
      windowrule = [
        "float,class:(clipse)"
        "size 45% 40%,class:(clipse)"
        "stayfocused, class:(clipse)"
        # copyq
        "float,class:(com.github.hluk.copyq)"
        "size 50% 50%,class:(com.github.hluk.copyq)"
        # Rofi
        "stayfocused, class:(Rofi)"
        "workspace 4 silent, class:(vesktop|legcord|discord)"

        # Fullscreen windows should be opaque
        "opacity 1.0 override,fullscreen:1"

        # Steam
        "workspace 3, class:steam_app_[0-9]+" # Move all steam games to workspace 3
        "idleinhibit always, class:steam_app_[0-9]+" # Always idle inhibit when playing a steam game
        # xwayland popups
        "nodim, xwayland:1, title:win[0-9]+"
        "noshadow, xwayland:1, title:win[0-9]+"
        "rounding 10, xwayland:1, title:win[0-9]+"
        # Dialogs
        "float, title:(Select|Open)( a)? (File|Folder)(s)?"
        "float, title:File (Operation|Upload)( Progress)?"
        "float, title:.* Properties"
        "float, title:Export Image as PNG"
        "float, title:GIMP Crash Debug"
        "float, title:Save As"
        "float, title:Library"
        # Float
        "float, class:org\.gnome\.FileRoller"
        "float, class:file-roller"
        "float, class:org.libvips.vipsdisp"
        "size 60% 70%,class:mpv"
        "float, class:mpv"
      ]
      ++ opaque [
        "zen-beta"
        "firefox"
        "chromium"
        "chrome"
        "mpv"
        "vlc"
        "youtube"
        "steam_app_[0-9]+"
        "lutris"
        "heroic"
        "plex"
      ]
      ++ opacity 0.9 [ ];
    };
  };
}
