{
  pkgs,
  config,
  lib,
  ...
}:
let

  clipboardScript = pkgs.writeScriptBin "clippaste" ''
    #!/bin/sh
    activeWindow=$(${pkgs.hyprland}/bin/hyprctl activewindow -j | ${pkgs.jq} -r '.class') # -r strips quotes
    shiftPasteClasses=(kitty)
    echo "Active window class: $activeWindow"
    contains_element() {
      local e
      for e in "$@:2"; do [[ "$e" == "$1" ]] && return 0; done
      return 1
    }
    if contains_element "$activeWindow" "$shiftPasteClasses[@]"; then
        ${pkgs.hyprland}/bin/hyprctl dispatch sendshortcut "CTRL SHIFT,V,"
    else
        ${pkgs.hyprland}/bin/hyprctl dispatch sendshortcut "CTRL,V,"
    fi
  '';

  kitty = {
    pkg = pkgs.kitty;
    bin = lib.getExe kitty.pkg;
  };

  defaultTerminal = kitty.bin;

  hyprlock = {
    pkg = config.programs.hyprlock.package;
    bin = lib.getExe hyprlock.pkg;
  };

  rofi = {
    pkg = pkgs.rofi;
    bin = lib.getExe rofi.pkg;
  };

  volume_script = lib.getExe pkgs.volume_script_hyprpanel;
  brightness_script = lib.getExe pkgs.brightness_script_hyprpanel;

  recShot = "${lib.getExe pkgs.recshot} -t ${
    config.sops.secrets."zipline/token".path
  } -u https://zipline.talos.plexuz.xyz -p ~/Pictures/Screenshots --zipline";

  mainMod = "SUPER";
  mainModShift = "${mainMod} SHIFT";

in
{
  wayland.windowManager.hyprland = {
    settings = {
      bind = [
        "${mainMod},ESCAPE,exec,${lib.getExe pkgs.wlogout}"
        "${mainMod},L,exec,${hyprlock.bin} --immediate"
        "${mainMod},R,exec,${rofi.bin} -show drun"

        # Applications
        "${mainMod},B,exec,${lib.getExe config.programs.zen-browser.package}"
        "${mainMod},E,exec,${lib.getExe pkgs.nautilus}"
        "${mainMod},RETURN,exec,${defaultTerminal}"
        "${mainModShift},RETURN,exec,[float] ${defaultTerminal}"
        "${mainMod},T,exec,${defaultTerminal}"
        "${mainModShift},T,exec,[float] ${defaultTerminal}"
        "${mainMod},O,exec,${lib.getExe pkgs.gnome-calculator}"
        "CTRL SHIFT,ESCAPE,exec,${lib.getExe pkgs.resources}"
        "${mainMod},C,exec,${defaultTerminal} --class clipse -e ${lib.getExe config.hmModules.desktop.clipse.package} && ${lib.getExe clipboardScript}"

        # Printscreen
        "${mainModShift},S,exec,${recShot} -m image-area"
        ",PRINT,exec,${recShot} -m image-full"
        "ALT,PRINT,exec,${recShot} -m image-window"
        "SHIFT ALT,S,exec,${recShot} -m video-area"
        "SHIFT,PRINT,exec,${recShot} -m video-window"

        # Hyprland binds
        "${mainMod},Q,killactive"
        "${mainMod},V,togglefloating"
        "${mainMod},P,pseudo"
        "${mainMod},J,togglesplit"
        "${mainMod},F,fullscreen,1"
        "${mainModShift},F,fullscreen,2"
        "${mainModShift},LEFT,movewindow,l"
        "${mainModShift},RIGHT,movewindow,r"
        "${mainModShift},UP,movewindow,u"
        "${mainModShift},DOWN,movewindow,d"

        # Move focus with mainMod + arrow keys
        "${mainMod},left,movefocus,l"
        "${mainMod},right,movefocus,r"
        "${mainMod},up,movefocus,u"
        "${mainMod},down,movefocus,d"

        # Switch workspaces with mainMod + [0-9]
        "${mainMod},1,workspace,1"
        "${mainMod},2,workspace,2"
        "${mainMod},3,workspace,3"
        "${mainMod},4,workspace,4"
        "${mainMod},5,workspace,5"
        "${mainMod},6,workspace,6"
        "${mainMod},7,workspace,7"
        "${mainMod},8,workspace,8"
        "${mainMod},9,workspace,9"
        "${mainMod},0,workspace,10"

        # Scratchpad
        "${mainMod},W,togglespecialworkspace"
        "${mainModShift},W,movetoworkspace,special"

        # Move active window to a workspace with mainMod + SHIFT + [0-9]
        "${mainModShift},1,movetoworkspace,1"
        "${mainModShift},2,movetoworkspace,2"
        "${mainModShift},3,movetoworkspace,3"
        "${mainModShift},4,movetoworkspace,4"
        "${mainModShift},5,movetoworkspace,5"
        "${mainModShift},6,movetoworkspace,6"
        "${mainModShift},7,movetoworkspace,7"
        "${mainModShift},8,movetoworkspace,8"
        "${mainModShift},9,movetoworkspace,9"
        "${mainModShift},0,movetoworkspace,10"

        # Scroll through existing workspaces with mainMod + scroll
        "${mainMod},mouse_down,workspace,e+1"
        "${mainMod},mouse_up,workspace,e-1"
      ];

      bindl = [
        # Audio
        ",XF86AudioMute,exec,${volume_script} mute"
        # Media keys
        ",XF86AudioPlay,exec,${lib.getExe pkgs.playerctl} play-pause"
        ",XF86AudioPrev,exec,${lib.getExe pkgs.playerctl} previous"
        ",XF86AudioNext,exec,${lib.getExe pkgs.playerctl} next"
      ];

      bindel = [
        # Brightness
        ",XF86MonBrightnessUp,   exec, ${brightness_script} up"
        ",XF86MonBrightnessDown, exec, ${brightness_script} down"

        # Audio
        ",XF86AudioRaiseVolume,  exec, ${volume_script} up"
        ",XF86AudioLowerVolume,  exec, ${volume_script} down"
      ];

      bindm = [
        # Move/resize windows with mainMod + SHIFT and dragging
        "${mainMod},       mouse:272, movewindow"
        "${mainModShift},  mouse:272, resizewindow"
      ];
    };
  };
}
