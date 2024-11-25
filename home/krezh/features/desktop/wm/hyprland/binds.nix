{
  inputs,
  pkgs,
  config,
  ...
}:
let
  defaultTerminal = "${pkgs.xdg-terminal-exec}/bin/xdg-terminal-exec";

  hyprlock = {
    pkg = config.programs.hyprlock.package;
    bin = "${hyprlock.pkg}/bin/hyprlock";
  };

  anyrun = {
    pkg = inputs.anyrun.packages.${pkgs.system};
    stdin = "${anyrun.pkg.stdin}/lib/libstdin.so";
    apps = "${anyrun.pkg.applications}/lib/libapplications.so";
    bin = "${anyrun.pkg.default}/bin/anyrun";
  };

  walker = {
    pkg = config.programs.walker.package;
    bin = "${walker.pkg}/bin/walker";
  };

  chrome = {
    pkg = inputs.browser-previews.packages.${pkgs.system}.google-chrome;
    bin = "${chrome.pkg}/bin/google-chrome-stable";
  };

  hyprkeys = {
    pkg = inputs.hyprkeys.packages.${pkgs.system}.hyprkeys;
    bin = "${hyprkeys.pkg}/bin/hyprkeys";
  };

  volume_script =
    if config.hmModules.desktop.hyprpanel.enable then
      "${pkgs.volume_script_hyprpanel}/bin/volume_script_hyprpanel"
    else
      "${pkgs.volume_script}/bin/volume_script";

  brightness_script =
    if config.hmModules.desktop.hyprpanel.enable then
      "${pkgs.brightness_script_hyprpanel}/bin/brightness_script_hyprpanel"
    else
      "${pkgs.brightness_script}/bin/brightness_script";

  grimblast = {
    pkg = inputs.hyprland-contrib.packages.${pkgs.system}.grimblast;
    bin = "${grimblast.pkg}/bin/grimblast";
  };

  mainMod = "SUPER";
  mainModShift = "${mainMod} SHIFT";

in
{
  wayland.windowManager.hyprland = {
    settings = {

      bind = [
        "${mainMod},ESCAPE,exec,${pkgs.wlogout}/bin/wlogout"
        "${mainMod},L,exec,${hyprlock.bin} --immediate"
        "${mainMod},R,exec,${walker.bin}"
        "${mainMod},K,exec,${hyprkeys.bin} -b -r | ${anyrun.bin} --plugins ${anyrun.stdin}"
        # Applications
        "${mainMod},B,exec,${chrome.bin}"
        "${mainMod},E,exec,${pkgs.nemo}/bin/nemo"
        "${mainMod},RETURN,exec,${defaultTerminal}"
        "${mainModShift},RETURN,exec,[floating] ${defaultTerminal}"
        "${mainMod},O,exec,${pkgs.obsidian}/bin/obsidian"
        "CTRL SHIFT,ESCAPE,exec,${pkgs.resources}/bin/resources"

        # Printscreen
        "ALT,P,exec,${grimblast.bin} --notify copy"
        "ALT SHIFT,P,exec,${grimblast.bin} --notify --freeze copy area || notify-send 'Grimblast'"

        # Audio
        ",XF86AudioMute,exec,${volume_script} mute"

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
        "${mainMod},S,togglespecialworkspace"
        "${mainModShift},S,movetoworkspace,special"

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

      binde = [
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
