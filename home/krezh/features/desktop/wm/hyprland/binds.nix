{
  inputs,
  pkgs,
  config,
  ...
}:
let
  hyprlock = {
    pkg = config.programs.hyprlock.package;
    bin = "${hyprlock.pkg}/bin/hyprlock";
  };

  clipman = {
    pkg = pkgs.clipman;
    bin = "${clipman.pkg}/bin/clipman";
  };

  anyrun = {
    pkg = inputs.anyrun.packages.${pkgs.system};
    stdin = "${anyrun.pkg.stdin}/lib/libstdin.so";
    apps = "${anyrun.pkg.applications}/lib/libapplications.so";
    bin = "${anyrun.pkg.default}/bin/anyrun";
  };

  chrome = {
    pkg = inputs.browser-previews.packages.${pkgs.system}.google-chrome;
    bin = "${chrome.pkg}/bin/google-chrome-stable";
  };

  hyprkeys = {
    pkg = inputs.hyprkeys.packages.${pkgs.system}.hyprkeys;
    bin = "${hyprkeys.pkg}/bin/hyprkeys";
  };

  wezterm = {
    pkg = config.programs.wezterm.package;
    bin = "${wezterm.pkg}/bin/wezterm";
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

in
{
  wayland.windowManager.hyprland = {
    settings = {
      "$mainMod" = "SUPER";
      "$SupShft" = "SUPER SHIFT";

      bind = [
        "$mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout"
        "$mainMod,L,exec,${hyprlock.bin} --immediate"
        "$mainMod,R,exec,${anyrun.bin} --plugins ${anyrun.apps}"
        "$mainMod,K,exec,${hyprkeys.bin} -b -r | ${anyrun.bin} --plugins ${anyrun.stdin}"
        # Applications
        "$mainMod,B,exec,${chrome.bin}"
        "$mainMod,E,exec,${pkgs.nemo}/bin/nemo"
        "$mainMod,RETURN,exec,${wezterm.bin}"
        "$SupShft,RETURN,exec,[floating] ${wezterm.bin}"
        "$mainMod,C,exec,${clipman.bin} pick -t rofi"
        "$mainMod,O,exec,${pkgs.obsidian}/bin/obsidian"

        # Print Screen
        "ALT,P,exec,${grimblast.bin} --notify copy"
        "ALT SHIFT,P,exec,${grimblast.bin} --notify --freeze copy area || notify-send 'Grimblast'"

        # Audio
        ",XF86AudioMute,exec,${volume_script} mute"

        # Hyprland binds
        "$mainMod,Q,killactive"
        "$mainMod,V,togglefloating"
        "$mainMod,P,pseudo"
        "$mainMod,J,togglesplit"
        "$mainMod,F,fullscreen,1"
        "$SupShft,F,fullscreen,2"
        "$SupShft,LEFT,movewindow,l"
        "$SupShft,RIGHT,movewindow,r"
        "$SupShft,UP,movewindow,u"
        "$SupShft,DOWN,movewindow,d"

        # Move focus with mainMod + arrow keys
        "$mainMod,left,movefocus,l"
        "$mainMod,right,movefocus,r"
        "$mainMod,up,movefocus,u"
        "$mainMod,down,movefocus,d"

        # Switch workspaces with mainMod + [0-9]
        "$mainMod,1,workspace,1"
        "$mainMod,2,workspace,2"
        "$mainMod,3,workspace,3"
        "$mainMod,4,workspace,4"
        "$mainMod,5,workspace,5"
        "$mainMod,6,workspace,6"
        "$mainMod,7,workspace,7"
        "$mainMod,8,workspace,8"
        "$mainMod,9,workspace,9"
        "$mainMod,0,workspace,10"

        # Scratchpad
        "$mainMod,S,togglespecialworkspace"
        "$mainModSHIFT,S,movetoworkspace,special"

        # Move active window to a workspace with mainMod + SHIFT + [0-9]
        "$mainModSHIFT,1,movetoworkspace,1"
        "$mainModSHIFT,2,movetoworkspace,2"
        "$mainModSHIFT,3,movetoworkspace,3"
        "$mainModSHIFT,4,movetoworkspace,4"
        "$mainModSHIFT,5,movetoworkspace,5"
        "$mainModSHIFT,6,movetoworkspace,6"
        "$mainModSHIFT,7,movetoworkspace,7"
        "$mainModSHIFT,8,movetoworkspace,8"
        "$mainModSHIFT,9,movetoworkspace,9"
        "$mainModSHIFT,0,movetoworkspace,10"

        # Scroll through existing workspaces with mainMod + scroll
        "$mainMod,mouse_down,workspace,e+1"
        "$mainMod,mouse_up,workspace,e-1"
        # Hyprland Plugins
        "$mainMod,TAB,hyprexpo:expo,toggle" # can be: toggle, off/disable or on/enable
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
        "$mainMod,       mouse:272, movewindow"
        "$mainMod SHIFT, mouse:272, resizewindow"
      ];
    };
  };
}
