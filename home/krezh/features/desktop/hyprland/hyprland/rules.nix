let
  # Simple helper to create opacity rules for a list of app classes
  opaque = apps: map (app: "opacity 1.0 override,class:^(${app})$") apps;
  opacity = value: apps: map (app: "opacity ${toString value} override,class:^(${app})$") apps;
in
{
  wayland.windowManager.hyprland = {
    settings = {
      windowrulev2 = [
        "float,class:(clipse)"
        "size 622 652,class:(clipse)"
        "stayfocused, class:(clipse)"
        "stayfocused, class:(Rofi)"
        "workspace 4 silent, class:^(vesktop|legcord|discord)$"
        "workspace 3, class:^(steam_app_[0-9]+)$"

        # Fullscreen windows should be opaque
        "opacity 1.0 override,fullscreen:1"
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
      ++ opacity 0.9 [
      ];
    };
  };
}
