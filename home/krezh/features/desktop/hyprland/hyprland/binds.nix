{
  pkgs,
  config,
  lib,
  ...
}:
let

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

  volume_script =
    if config.programs.hyprpanel.enable then
      lib.getExe pkgs.volume_script_hyprpanel
    else
      lib.getExe pkgs.volume_script;

  brightness_script =
    if config.programs.hyprpanel.enable then
      lib.getExe pkgs.brightness_script_hyprpanel
    else
      lib.getExe pkgs.brightness_script;

  recShot = "${lib.getExe pkgs.zipline-recshot} -t ${
    config.sops.secrets."zipline/token".path
  } -u https://zipline.talos.plexuz.xyz -p ~/Pictures/Screenshots";

  mainMod = "SUPER";
  mainModShift = "${mainMod} SHIFT";

in
{
  wayland.windowManager.hyprland = {
    settings = {
      windowrulev2 = [
        "float,class:(clipse)"
        "size 622 652,class:(clipse)"
        "stayfocused, class:(clipse)"
        "stayfocused, class:(Rofi)"
        "workspace 4 silent, class:^(legcord)$"
        "workspace 3, class:^(steam_app_[0-9]+)$"
      ];

      bind = [
        "${mainMod},ESCAPE,exec,${lib.getExe pkgs.wlogout}"
        "${mainMod},L,exec,${hyprlock.bin} --immediate"
        "${mainMod},R,exec,${rofi.bin} -show drun"
        # Applications
        "${mainMod},B,exec,${config.home.sessionVariables.DEFAULT_BROWSER}"
        "${mainMod},E,exec,${lib.getExe pkgs.nautilus}"
        "${mainMod},RETURN,exec,${defaultTerminal}"
        "${mainModShift},RETURN,exec,[float] ${defaultTerminal}"
        "${mainMod},O,exec,${lib.getExe pkgs.gnome-calculator}"
        "CTRL SHIFT,ESCAPE,exec,${lib.getExe pkgs.resources}"
        "${mainMod},C,exec,${defaultTerminal} --class clipse ${lib.getExe config.hmModules.desktop.clipse.package}"

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
