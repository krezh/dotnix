{
  flake.modules.homeManager.hyprland =
    {
      pkgs,
      config,
      lib,
      ...
    }:
    let
      mkProg = pkg: {
        run = lib.getExe pkg;
        name = pkg.meta.mainProgram or pkg.pname or pkg.name;
      };
      mkProgWith = pkg: args: mkProg pkg // { run = "${lib.getExe pkg} ${args}"; };

      term =
        let
          base = lib.getExe pkgs.ghostty;
        in
        {
          run = "${base} +new-window";
          float = cmd: "${base} --class=floatTerm -e ${cmd}";
          toggle = proc: cmd: "pkill ${proc} || ${base} --class=floatTerm -e ${cmd}";
        };

      browser.run = "${lib.getExe config.programs.wlr-which-key.package} browser";
      screenshot.run = "${lib.getExe config.programs.wlr-which-key.package} screenshot";
      fileManager = mkProg pkgs.nautilus;
      passwords = mkProg pkgs.proton-pass;
      sysMonitor = mkProg pkgs.resources;
      logout.run = "noctalia-shell ipc call sessionMenu toggle";
      hyprlock.run = "${lib.getExe config.programs.hyprlock.package} --immediate";
      launcher.run = "${pkgs.netcat}/bin/nc -U /run/user/$EUID/walker/walker.sock";
      shell.run = "${lib.getExe config.programs.noctalia-shell.package} ipc call";
      keybinds.run = lib.getExe pkgs.hyprland_keybinds;
      clipboardMgr.run = "${lib.getExe config.programs.walker.package} -m clipboard";
      mail.run = lib.getExe' pkgs.geary "geary";
      audioControl = mkProgWith pkgs.pwvucontrol "--tab 4";
      trayTui = mkProg pkgs.tray-tui;
      volume_script = lib.getExe pkgs.volume_script_hyprpanel;
      brightness_script = lib.getExe pkgs.brightness_script_hyprpanel;

      mainMod = "SUPER";
      mainModShift = "${mainMod} SHIFT";
    in
    {
      wayland.windowManager.hyprland = {
        settings = {
          "$mainMod" = "${mainMod}";
          bindd = [
            "${mainMod},ESCAPE,Logout Menu,exec,${logout.run}"
            "${mainMod},L,Lockscreen,exec,${hyprlock.run}"
            "${mainMod},R,Application launcher,exec,${launcher.run}"
            "${mainMod},N,Notifications,exec,${shell.run} notifications toggleHistory"
            "${mainModShift},N,Clear notifications,exec,${shell.run} notifications clear"
            "${mainMod},B,Browser,exec,${browser.run}"
            "${mainMod},E,File Manager,exec,${fileManager.run}"
            "${mainMod},P,Passwords,exec,${passwords.run}"
            "${mainMod},RETURN,Terminal,exec,${term.run}"
            "${mainModShift},RETURN,Terminal,exec,[float] ${term.run}"
            "${mainMod},T,Tray-Tui,exec,[float] ${term.toggle trayTui.name trayTui.run}"
            "CTRL SHIFT,ESCAPE,System Monitor,exec,[float] ${sysMonitor.run}"
            "${mainMod},V,Clipboard Manager,exec,${clipboardMgr.run}"
            "${mainMod},K,Show keybinds,exec,${keybinds.run}"
            "${mainMod},G,Audio Control,exec,[float] ${audioControl.run}"
            "${mainMod},M,Mail Client,exec,${mail.run}"
            # "${mainMod},TAB,Toggle workspace overview, hyprexpo:expo, toggle"
            "${mainModShift},R, Reload Hyprland config,exec,hyprctl reload && notify-send --transient -u low 'Hyprland' 'Config Reloaded'"
            "${mainMod},A,Toggle between audio devices,exec,audio-switch toggle"
            "${mainMod},S,Screenshot menu,exec,${screenshot.run}"
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
            "${mainModShift},mouse:272,Resize window with ${mainModShift} + left mouse drag,resizewindow"
          ];
        };
      };
    };
}
