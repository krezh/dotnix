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
  getBinaryName = pkg: pkg.meta.mainProgram or pkg.pname or pkg.name;

  kitty = {
    pkg = pkgs.kitty;
    bin = lib.getExe kitty.pkg;
  };

  defaultTerminal = kitty.bin;

  hyprlock = {
    pkg = config.programs.hyprlock.package;
    bin = lib.getExe hyprlock.pkg;
  };

  launcher = {
    pkg = pkgs.fuzzel;
    bin = lib.getExe launcher.pkg;
    name = getBinaryName launcher.pkg;
  };

  showkey = {
    pkg = pkgs.hypr-showkey;
    bin = lib.getExe showkey.pkg;
  };

  audioControl = {
    pkg = pkgs.wiremix;
    bin = lib.getExe audioControl.pkg;
    name = getBinaryName audioControl.pkg;
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
      bindd = [
        "${mainMod},ESCAPE,Show logout menu,exec,${lib.getExe pkgs.wlogout}"
        "${mainMod},L,Lock the screen immediately,exec,${hyprlock.bin} --immediate"
        "${mainMod},R,Launch application launcher,exec,pkill ${launcher.name} || ${launcher.bin}"
        "${mainMod},B,Launch Zen Browser,exec,${lib.getExe config.programs.zen-browser.package}"
        "${mainMod},E,Launch Nautilus file manager,exec,${lib.getExe pkgs.nautilus}"
        "${mainMod},RETURN,Launch terminal,exec,${defaultTerminal}"
        "${mainModShift},RETURN,Launch terminal (floating),exec,[float] ${defaultTerminal}"
        "${mainMod},T,Launch terminal,exec,${defaultTerminal}"
        "${mainModShift},T,Launch terminal (floating),exec,[float] ${defaultTerminal}"
        "${mainMod},O,Launch calculator,exec,${lib.getExe pkgs.gnome-calculator}"
        "CTRL SHIFT,ESCAPE,Launch system resources monitor (floating),exec,[float] ${lib.getExe pkgs.resources}"
        "${mainMod},C,Launch Clipse clipboard manager in terminal and run clipboard script,exec,${defaultTerminal} --class clipse -e ${lib.getExe config.hmModules.desktop.clipse.package} && ${lib.getExe clipboardScript}"
        "${mainMod},K,Show keybinds (floating),exec,[float] ${defaultTerminal} --class showkey -e ${showkey.bin}"
        "${mainMod},G,Launch Audio Control (floating),exec,[float] pkill ${audioControl.name} || ${defaultTerminal} --class audioControl -e ${audioControl.bin} -m 100 "

        "${mainModShift},S,Area screenshot,exec,${recShot} -m image-area"
        ",PRINT,Fullscreen screenshot,exec,${recShot} -m image-full"
        "ALT,PRINT,Window screenshot,exec,${recShot} -m image-window"
        "SHIFT ALT,S,Area screen recording,exec,${recShot} -m video-area"
        "SHIFT,PRINT,Window screen recording,exec,${recShot} -m video-window"

        "${mainMod},Q,Close active window,killactive"
        "${mainMod},V,Toggle floating mode,togglefloating"
        "${mainMod},P,Toggle pseudo tiling,pseudo"
        "${mainMod},J,Toggle split layout,togglesplit"
        "${mainMod},F,Toggle fullscreen,fullscreen,1"
        "${mainModShift},F,Toggle fullscreen,fullscreen,2"
        "${mainModShift},LEFT,Move window left,movewindow,l"
        "${mainModShift},RIGHT,Move window right,movewindow,r"
        "${mainModShift},UP,Move window up,movewindow,u"
        "${mainModShift},DOWN,Move window down,movewindow,d"

        "${mainMod},left,Move focus left,movefocus,l"
        "${mainMod},right,Move focus right,movefocus,r"
        "${mainMod},up,Move focus up,movefocus,u"
        "${mainMod},down,Move focus down,movefocus,d"

        "${mainMod},1,Switch to workspace 1,workspace,1"
        "${mainMod},2,Switch to workspace 2,workspace,2"
        "${mainMod},3,Switch to workspace 3,workspace,3"
        "${mainMod},4,Switch to workspace 4,workspace,4"
        "${mainMod},5,Switch to workspace 5,workspace,5"
        "${mainMod},6,Switch to workspace 6,workspace,6"
        "${mainMod},7,Switch to workspace 7,workspace,7"
        "${mainMod},8,Switch to workspace 8,workspace,8"
        "${mainMod},9,Switch to workspace 9,workspace,9"
        "${mainMod},0,Switch to workspace 10,workspace,10"

        "${mainMod},W,Toggle special workspace,togglespecialworkspace"
        "${mainModShift},W,Move window to special workspace,movetoworkspace,special"

        "${mainModShift},1,Move active window to workspace 1,movetoworkspace,1"
        "${mainModShift},2,Move active window to workspace 2,movetoworkspace,2"
        "${mainModShift},3,Move active window to workspace 3,movetoworkspace,3"
        "${mainModShift},4,Move active window to workspace 4,movetoworkspace,4"
        "${mainModShift},5,Move active window to workspace 5,movetoworkspace,5"
        "${mainModShift},6,Move active window to workspace 6,movetoworkspace,6"
        "${mainModShift},7,Move active window to workspace 7,movetoworkspace,7"
        "${mainModShift},8,Move active window to workspace 8,movetoworkspace,8"
        "${mainModShift},9,Move active window to workspace 9,movetoworkspace,9"
        "${mainModShift},0,Move active window to workspace 10,movetoworkspace,10"

        "${mainMod},mouse_down,Next workspace,workspace,e+1"
        "${mainMod},mouse_up,Previous workspace,workspace,e-1"
      ];

      binddl = [
        ",XF86AudioMute,Toggle mute,exec,${volume_script} mute"
        ",XF86AudioPlay,Play/pause media,exec,${lib.getExe pkgs.playerctl} play-pause"
        ",XF86AudioPrev,Previous track,exec,${lib.getExe pkgs.playerctl} previous"
        ",XF86AudioNext,Next track,exec,${lib.getExe pkgs.playerctl} next"
      ];

      binddel = [
        ",XF86MonBrightnessUp,Increase brightness,exec,${brightness_script} up"
        ",XF86MonBrightnessDown,Decrease brightness,exec,${brightness_script} down"
        ",XF86AudioRaiseVolume,Increase volume,exec,${volume_script} up"
        ",XF86AudioLowerVolume,Decrease volume,exec,${volume_script} down"
      ];

      binddm = [
        "${mainMod},mouse:272,Move window with ${mainMod} + left mouse drag,movewindow"
        "${mainModShift},mouse:272,Resize window with ${mainMod} + SHIFT + left mouse drag,resizewindow"
      ];
    };
  };
}
