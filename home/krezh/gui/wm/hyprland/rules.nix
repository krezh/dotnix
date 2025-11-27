{ var, ... }:
{
  wayland.windowManager.hyprland = {
    settings = {
      layerrule = [
        # Rofi
        "blur,rofi"
        # Fuzzel
        "animation popin 80%, launcher"
        "blur, launcher"
        # Walker
        "animation popin 60%, walker"
        # Hyprpicker
        "animation fade, hyprpicker"
        # Wlogout
        "animation fade, logout_dialog"
        # Slurp
        "animation fade, selection"
        # Wayfreeze
        "animation fade, wayfreeze"
        # Noctalia Shell
        "noanim, (noctalia:.*)"
      ];
      windowrule = [
        # Rofi
        "stayfocused, class:(Rofi)"
        # Chat workspace
        "workspace 4 silent, tag:chat"
        # Fullscreen opacity
        "opacity 1.0 override,fullscreen:1"
        # Games workspace and idle inhibit
        "workspace 3, tag:games"
        "idleinhibit always, tag:games"
        "idleinhibit fullscreen, fullscreen:1"
        # XWayland popups
        "nodim, xwayland:1, title:win[0-9]+"
        "noshadow, xwayland:1, title:win[0-9]+"
        "rounding ${toString var.rounding}, xwayland:1, title:win[0-9]+"
        # Dialog windows
        "float, title:(Select|Open)( a)? (File|Folder)(s)?"
        "float, title:File (Operation|Upload)( Progress)?"
        "float, title:.* Properties"
        "float, title:Export Image as PNG"
        "float, title:GIMP Crash Debug"
        "float, title:Save As"
        "float, title:Library"
        "float, title:Install, class:steam"
        "size 50% 50%, title:Install, class:steam"
        # File managers
        "float, class:org\.gnome\.FileRoller"
        "float, class:file-roller"
        # Vips image viewer
        "float, class:org\.libvips\.vipsdisp"
        # MPV
        "float, class:mpv"
        "size 60% 70%,class:mpv"
        # Float Terminal
        "float, class:floatTerm"
        "size 60% 60%, class:floatTerm"
        # Tag games
        "tag +games, class:^(gamescope)$"
        "tag +games, class:^(steam_proton)$"
        "tag +games, class:^(steam_app_default)$"
        "tag +games, class:^(steam_app_[0-9]+)$"
        # Tag browsers
        "tag +browsers, class:^(zen.*)$"
        "tag +browsers, class:^(firefox)$"
        "tag +browsers, class:^(chromium)$"
        "tag +browsers, class:^(chrome)$"
        # Tag media
        "tag +media, class:^(mpv)$"
        "tag +media, class:^(vlc)$"
        "tag +media, class:^(youtube)$"
        "tag +media, class:^(plex)$"
        # Tag chat
        "tag +chat, class:^(vesktop)$"
        "tag +chat, class:^(legcord)$"
        "tag +chat, class:^(discord)$"
        # Opacity overrides
        "opacity 1.0 override,tag:games"
        "opacity 1.0 override,tag:browsers"
        "opacity 1.0 override,tag:media"
        "opacity 1.0 override,initialTitle:^(Discord Popout)$"
        # Blur exceptions
        "noblur,tag:games"
        # Render unfocused
        "renderunfocused,tag:games"
      ];
    };
  };
}
