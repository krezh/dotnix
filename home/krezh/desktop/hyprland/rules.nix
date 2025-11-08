{ pkgs, ... }:
let
  opacity =
    value: items:
    map (
      item:
      if builtins.match "^tag:(.+)" item != null then
        "opacity ${toString value} override,tag:${builtins.elemAt (builtins.match "^tag:(.+)" item) 0}"
      else
        "opacity ${toString value} override,class:^(${item})$"
    ) items;

  tags = tag: apps: map (app: "tag +${tag}, class:^(${app})$") apps;

  noblur =
    items:
    map (
      item:
      if builtins.match "^tag:(.+)" item != null then
        "noblur,tag:${builtins.elemAt (builtins.match "^tag:(.+)" item) 0}"
      else
        "noblur,class:^(${item})$"
    ) items;

  renderunfocused =
    items:
    map (
      item:
      if builtins.match "^tag:(.+)" item != null then
        "renderunfocused,tag:${builtins.elemAt (builtins.match "^tag:(.+)" item) 0}"
      else
        "renderunfocused,class:^(${item})$"
    ) items;
  #from https://github.com/hyprwm/Hyprland/issues/3835
  float_script = pkgs.writeShellScriptBin "hyprland-bitwarden-float" ''
    handle() {
      case $1 in
        windowtitle*)
          window_id=''${1#*>>}
          [[ "$window_id" =~ ^0x ]] || window_id="0x$window_id"
          window_title=$(hyprctl clients -j | ${pkgs.jq}/bin/jq --arg id "$window_id" -r '.[] | select(.address == $id) | .title')
          if [[ "$window_title" == "Extension: (Bitwarden Password Manager) - Bitwarden — Zen Browser" ]]; then
            hyprctl --batch "
              dispatch togglefloating address:$window_id ;
              dispatch resizewindowpixel exact 20% 40%,address:$window_id ;
              dispatch movewindowpixel exact 40% 30%,address:$window_id
            "
          fi
          ;;
      esac
    }
    # Listen to the Hyprland socket for events and process each line with the handle function
    ${pkgs.socat}/bin/socat -U - UNIX-CONNECT:$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock | while read -r line; do handle "$line"; done
  '';
in
{
  home.packages = [ float_script ];
  wayland.windowManager.hyprland = {
    extraConfig = ''
      exec-once = ${float_script}/bin/hyprland-bitwarden-float
    '';
    settings = {
      layerrule = [
        "blur,rofi"
        # Fuzzel
        "animation popin 80%, launcher"
        "blur, launcher"
        # Walker
        "animation popin 80%, walker"
        "animation fade, hyprpicker" # Colour picker out animation
        "animation fade, logout_dialog" # wlogout
        "animation fade, selection" # slurp
        "animation fade, wayfreeze"
        # DankMaterialShell
        "noanim, (dms:.*)"
      ];
      windowrule = [
        "noshadow, focus:0"
        # Rofi
        "stayfocused, class:(Rofi)"
        # Chat
        "workspace 4 silent, tag:chat"
        # Fullscreen windows should be opaque
        "opacity 1.0 override,fullscreen:1"
        # Games
        "workspace 3, tag:games" # Move all games to workspace 3
        "idleinhibit always, tag:games" # Always idle inhibit when playing a game
        "idleinhibit fullscreen, fullscreen:1"
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
        "float, class:org\.libvips\.vipsdisp"
        "float, title:^(.*Bitwarden Password Manager.*)$"
        "size 50% 50%,title:^(.*Bitwarden Password Manager.*)$"
        "size 60% 70%,class:mpv"
        "float, class:mpv"
      ]
      ++ tags "games" [
        "gamescope"
        "steam_proton"
        "steam_app_default"
        "steam_app_[0-9]+"
      ]
      ++ tags "browsers" [
        "zen-beta"
        "firefox"
        "chromium"
        "chrome"
      ]
      ++ tags "media" [
        "mpv"
        "vlc"
        "youtube"
        "plex"
      ]
      ++ tags "chat" [
        "vesktop"
        "legcord"
        "discord"
      ]
      ++ opacity 1.0 [
        "tag:games"
        "tag:browsers"
        "tag:media"
      ]
      ++ noblur [
        "tag:games"
      ]
      ++ renderunfocused [
        "tag:games"
      ];
    };
  };
}
