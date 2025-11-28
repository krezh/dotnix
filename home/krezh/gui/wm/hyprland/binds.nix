{
  pkgs,
  config,
  lib,
  ...
}:
let
  getBinaryName = pkg: pkg.meta.mainProgram or pkg.pname or pkg.name;

  ghostty = {
    pkg = pkgs.ghostty;
    bin = lib.getExe ghostty.pkg;
  };

  defaultTerminal = ghostty.bin;

  hyprlock = {
    pkg = config.programs.hyprlock.package;
    bin = lib.getExe hyprlock.pkg;
  };

  launcher = {
    bin = "${pkgs.netcat}/bin/nc -U /run/user/$(id -u)/walker/walker.sock";
  };

  shell = {
    pkg = config.programs.noctalia-shell.package;
    bin = "${lib.getExe shell.pkg} ipc call";
  };

  keybinds = {
    pkg = pkgs.hyprland_keybinds;
    bin = lib.getExe keybinds.pkg;
  };

  audioControl = rec {
    pkg = pkgs.wiremix;
    bin = lib.getExe pkg;
    name = getBinaryName pkg;
  };

  trayTui = rec {
    pkg = pkgs.tray-tui;
    bin = lib.getExe pkg;
    name = getBinaryName pkg;
  };

  clipboardMgr = {
    pkg = config.programs.walker.package;
    bin = "${lib.getExe clipboardMgr.pkg} -m clipboard";
  };

  mail = {
    pkg = pkgs.geary;
    bin = "${mail.pkg}/bin/geary";
  };

  volume_script = lib.getExe pkgs.volume_script_hyprpanel;
  brightness_script = lib.getExe pkgs.brightness_script_hyprpanel;

  recShot = "${lib.getExe pkgs.recshot} -t ${
    config.sops.secrets."zipline/token".path
  } -u https://zipline.talos.plexuz.xyz --zipline";

  mainMod = "SUPER";
  mainModShift = "${mainMod} SHIFT";

in
{
  wayland.windowManager.hyprland = {
    settings = {
      bindd = [
        "${mainMod},ESCAPE,Show logout menu,exec,${lib.getExe pkgs.wlogout}"
        "${mainMod},L,Lock the screen immediately,exec,${hyprlock.bin} --immediate"
        "${mainMod},R,Launch application launcher,exec,${launcher.bin}"
        "${mainMod},N,Launch notifications,exec,${shell.bin} notifications toggleHistory"
        "${mainMod} CTRL,N,Clear notifications,exec,${shell.bin} notifications clear"
        "${mainMod},B,Launch Zen Browser,exec,${lib.getExe config.programs.zen-browser.package}"
        "${mainMod},E,Launch Nautilus file manager,exec,${lib.getExe pkgs.nautilus}"
        "${mainModShift},E,Launch Nautilus file manager in floating mode,exec,[float] ${lib.getExe pkgs.nautilus}"
        "${mainMod},P,Launch bitwarden,exec,${lib.getExe pkgs.bww}"
        "${mainMod},RETURN,Launch terminal,exec,${defaultTerminal}"
        "${mainModShift},RETURN,Launch terminal,exec,[float] ${defaultTerminal}"
        "${mainMod},T,Launch tray-tui,exec,[float] pkill ${trayTui.name} || ${defaultTerminal} --class=com.example.floatterm -e ${trayTui.bin}"
        "CTRL SHIFT,ESCAPE,Launch system resources monitor,exec,[float] ${lib.getExe pkgs.mission-center}"
        "${mainMod},V,Launch clipboard manager,exec,${clipboardMgr.bin}"
        "${mainMod},K,Show keybinds,exec,${keybinds.bin}"
        "${mainMod},G,Launch Audio Control,exec,[float] pkill ${audioControl.name} || ${defaultTerminal} --class=com.example.floatterm -e ${audioControl.bin} -m 100"
        "${mainMod},M,Launch Default Mail Client,exec,${mail.bin}"
        # HyprExpo workspace overview
        "${mainMod},TAB,Toggle workspace overview, hyprexpo:expo, toggle"
        # Audio device switching
        "${mainMod},F3,Toggle between audio devices,exec,audio-switch toggle"
        # Screenshots and screen recordings
        "${mainModShift},S,Area screenshot,exec,${recShot} -m image-area"
        ",PRINT,Fullscreen screenshot,exec,${recShot} -m image-full"
        "ALT,PRINT,Window screenshot,exec,${recShot} -m image-window"
        "SHIFT ALT,S,Area screen recording,exec,${recShot} -m video-area"
        "SHIFT,PRINT,Window screen recording,exec,${recShot} -m video-window"

        # Window management
        "${mainMod},Q,Close active window,killactive"
        "${mainMod},C,Toggle floating mode,togglefloating"
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
        "${mainMod},mouse:273,Resize window with ${mainMod} + right mouse drag,resizewindow"
      ];
    };
  };
}
