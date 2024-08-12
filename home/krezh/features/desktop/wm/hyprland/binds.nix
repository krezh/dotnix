{
  inputs,
  pkgs,
  config,
  ...
}:
let
  hyprlockConfig = config.programs.hyprlock.package;
  anyrunFlake = inputs.anyrun.packages.${pkgs.system};
  anyrun = {
    stdin = "${anyrunFlake.stdin}/lib/libstdin.so";
    applications = "${anyrunFlake.applications}/lib/libapplications.so";
    bin = "${anyrunFlake.default}/bin/anyrun";
  };
  chromeFlake = inputs.browser-previews.packages.${pkgs.system}.google-chrome;
  chrome = {
    bin = "${chromeFlake}/bin/google-chrome-stable";
  };
  hyprkeysFlake = inputs.hyprkeys.packages.${pkgs.system}.hyprkeys;
  weztermConfig = config.programs.wezterm.package;

  volume_script =
    if config.modules.desktop.hyprpanel.enable then
      "${pkgs.volume_script_hyprpanel}/bin/volume_script_hyprpanel"
    else
      "${pkgs.volume_script}/bin/volume_script";

  brightness_script =
    if config.modules.desktop.hyprpanel.enable then
      "${pkgs.brightness_script_hyprpanel}/bin/brightness_script_hyprpanel"
    else
      "${pkgs.brightness_script}/bin/brightness_script";

in
{
  wayland.windowManager.hyprland = {
    settings = {
      "$mainMod" = "SUPER";
      "$SupShft" = "SUPER SHIFT";

      bind = [
        "$mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout"
        "$mainMod,L,exec,${hyprlockConfig}/bin/hyprlock"
        "$mainMod,R,exec,${anyrunFlake.default}/bin/anyrun --plugins ${anyrun.applications}"
        "$mainMod,K,exec,${hyprkeysFlake}/bin/hyprkeys -b -r | anyrun --plugins ${anyrun.stdin}"
        # Applications
        "$mainMod,B,exec,${chrome.bin}"
        "$mainMod,E,exec,${pkgs.nemo}/bin/nemo"
        "$mainMod,RETURN,exec,${weztermConfig}/bin/wezterm"
        "$SupShft,RETURN,exec,${weztermConfig}/bin/wezterm"
        "$mainMod,C,exec,${pkgs.clipman}/bin/clipman pick -t rofi"
        "$mainMod,O,exec,${pkgs.obsidian}/bin/obsidian"

        # Print Screen
        "ALT,P,exec,grimshot copy"
        "ALT SHIFT,P,exec,pkill slurp || grimshot copy area"

        # Audio
        ",XF86AudioMute,exec,${volume_script}/bin/volume_script mute"

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
