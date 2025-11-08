{
  pkgs,
  config,
  lib,
  ...
}:
let
  kittyBin = lib.getExe pkgs.kitty;
  launcherBin = lib.getExe config.programs.walker.package;
  shellBin = "${lib.getExe config.programs.noctalia-shell.package} ipc call";
  clipboardBin = "${lib.getExe config.programs.walker.package} -m clipboard";
  recShot = "${lib.getExe pkgs.recshot} -t ${
    config.sops.secrets."zipline/token".path
  } -u https://zipline.talos.plexuz.xyz --zipline";

  getBinaryName = pkg: pkg.meta.mainProgram or pkg.pname or pkg.name;
  audioControlPkg = pkgs.wiremix;
  audioControlBin = lib.getExe audioControlPkg;
  audioControlName = getBinaryName audioControlPkg;

  actions = config.lib.niri.actions;
in
{
  programs.niri.settings.binds = with config.lib.niri.actions; {
    # Hotkey overlay
    "Mod+Shift+Slash".action = actions."show-hotkey-overlay";
    "Mod+Slash".action = actions."show-hotkey-overlay";

    # Overview
    "Mod+Tab".action = actions."toggle-overview";

    # Mouse wheel bindings for workspace navigation
    "Mod+WheelScrollDown" = {
      action = actions."focus-workspace-down";
      cooldown-ms = 150;
    };
    "Mod+WheelScrollUp" = {
      action = actions."focus-workspace-up";
      cooldown-ms = 150;
    };
    "Mod+WheelScrollRight".action = actions."focus-column-right";
    "Mod+WheelScrollLeft".action = actions."focus-column-left";

    # Mouse wheel for window navigation
    "Mod+Shift+WheelScrollDown".action = actions."focus-window-down";
    "Mod+Shift+WheelScrollUp".action = actions."focus-window-up";
    "Mod+Shift+WheelScrollRight".action = actions."focus-column-right";
    "Mod+Shift+WheelScrollLeft".action = actions."focus-column-left";

    # Mouse wheel for column width
    "Mod+Ctrl+WheelScrollDown" = {
      action = actions."set-column-width" "-10%";
      cooldown-ms = 150;
    };
    "Mod+Ctrl+WheelScrollUp" = {
      action = actions."set-column-width" "+10%";
      cooldown-ms = 150;
    };

    # Application Launchers
    "Mod+Return".action = spawn kittyBin;
    "Mod+T".action = spawn kittyBin;
    "Mod+R".action = spawn launcherBin;
    "Mod+B".action = spawn (lib.getExe config.programs.zen-browser.package);
    "Mod+E".action = spawn (lib.getExe pkgs.nautilus);
    "Mod+O".action = spawn (lib.getExe pkgs.gnome-calculator);
    "Mod+V".action = spawn "sh" "-c" clipboardBin;
    "Mod+N".action = spawn shellBin "notifications" "toggleHistory";
    "Mod+Ctrl+N".action = spawn shellBin "notifications" "clear";
    "Mod+Escape".action = spawn (lib.getExe pkgs.wlogout);
    "Ctrl+Shift+Escape".action = spawn (lib.getExe pkgs.resources);
    "Mod+G".action =
      spawn "sh" "-c"
        "pkill ${audioControlName} || ${kittyBin} --class audioControl -e ${audioControlBin} -m 100";

    # Window Management
    "Mod+Q".action = actions."close-window";
    "Mod+C".action = actions."toggle-window-floating";
    "Mod+F".action = actions."maximize-column";
    "Mod+Shift+F".action = actions."fullscreen-window";

    # Focus Movement
    "Mod+Left".action = actions."focus-column-left";
    "Mod+Right".action = actions."focus-column-right";
    "Mod+Up".action = actions."focus-window-up";
    "Mod+Down".action = actions."focus-window-down";

    # Window Movement
    "Mod+Shift+Left".action = actions."move-column-left";
    "Mod+Shift+Right".action = actions."move-column-right";
    "Mod+Shift+Up".action = actions."move-window-up";
    "Mod+Shift+Down".action = actions."move-window-down";

    # Workspace Navigation
    # Using named workspaces to bind to specific monitors (1-3 on DP-1, 4-6 on DP-2)
    "Mod+1".action = actions."focus-workspace" "1";
    "Mod+2".action = actions."focus-workspace" "2";
    "Mod+3".action = actions."focus-workspace" "3";
    "Mod+4".action = actions."focus-workspace" "4";
    "Mod+5".action = actions."focus-workspace" "5";
    "Mod+6".action = actions."focus-workspace" "6";
    "Mod+7".action = actions."focus-workspace" "7";
    "Mod+8".action = actions."focus-workspace" "8";
    "Mod+9".action = actions."focus-workspace" "9";
    "Mod+0".action = actions."focus-workspace" "10";

    # Move to Workspace
    # Using named workspaces to bind to specific monitors (1-3 on DP-1, 4-6 on DP-2)
    "Mod+Shift+1".action = {
      "move-column-to-workspace" = "1";
    };
    "Mod+Shift+2".action = {
      "move-column-to-workspace" = "2";
    };
    "Mod+Shift+3".action = {
      "move-column-to-workspace" = "3";
    };
    "Mod+Shift+4".action = {
      "move-column-to-workspace" = "4";
    };
    "Mod+Shift+5".action = {
      "move-column-to-workspace" = "5";
    };
    "Mod+Shift+6".action = {
      "move-column-to-workspace" = "6";
    };
    "Mod+Shift+7".action = {
      "move-column-to-workspace" = "7";
    };
    "Mod+Shift+8".action = {
      "move-column-to-workspace" = "8";
    };
    "Mod+Shift+9".action = {
      "move-column-to-workspace" = "9";
    };
    "Mod+Shift+0".action = {
      "move-column-to-workspace" = "10";
    };

    # Workspace Switching
    "Mod+Page_Down".action = actions."focus-workspace-down";
    "Mod+Page_Up".action = actions."focus-workspace-up";
    "Mod+Ctrl+Down".action = actions."focus-workspace-down";
    "Mod+Ctrl+Up".action = actions."focus-workspace-up";

    # Move to Monitor
    "Mod+Shift+Ctrl+Left".action = actions."move-column-to-monitor-left";
    "Mod+Shift+Ctrl+Right".action = actions."move-column-to-monitor-right";
    "Mod+Shift+Ctrl+Up".action = actions."move-column-to-monitor-up";
    "Mod+Shift+Ctrl+Down".action = actions."move-column-to-monitor-down";

    # Focus monitor
    "Mod+Ctrl+Left".action = actions."focus-monitor-left";
    "Mod+Ctrl+Right".action = actions."focus-monitor-right";
    "Mod+Ctrl+H".action = actions."focus-monitor-left";
    "Mod+Ctrl+L".action = actions."focus-monitor-right";

    # Column Width Adjustment
    "Mod+Minus".action = actions."set-column-width" "-10%";
    "Mod+Equal".action = actions."set-column-width" "+10%";
    "Mod+Shift+Minus".action = actions."set-window-height" "-10%";
    "Mod+Shift+Equal".action = actions."set-window-height" "+10%";

    # Column Width Presets
    "Mod+Shift+R".action = actions."reset-window-height";
    "Mod+W".action = actions."switch-preset-column-width";

    # Screenshots with recshot
    "Mod+Shift+S".action = spawn "sh" "-c" "${recShot} -m image-area";
    "Print".action = spawn "sh" "-c" "${recShot} -m image-full";
    "Alt+Print".action = spawn "sh" "-c" "${recShot} -m image-window";

    # Screen recordings with recshot
    "Shift+Alt+S".action = spawn "sh" "-c" "${recShot} -m video-area";
    "Shift+Print".action = spawn "sh" "-c" "${recShot} -m video-window";

    # Media Keys
    "XF86AudioRaiseVolume".action = spawn "wpctl" "set-volume" "@DEFAULT_AUDIO_SINK@" "5%+";
    "XF86AudioLowerVolume".action = spawn "wpctl" "set-volume" "@DEFAULT_AUDIO_SINK@" "5%-";
    "XF86AudioMute".action = spawn "wpctl" "set-mute" "@DEFAULT_AUDIO_SINK@" "toggle";
    "XF86AudioPlay".action = spawn (lib.getExe pkgs.playerctl) "play-pause";
    "XF86AudioPrev".action = spawn (lib.getExe pkgs.playerctl) "previous";
    "XF86AudioNext".action = spawn (lib.getExe pkgs.playerctl) "next";

    # Brightness Control
    "XF86MonBrightnessUp".action = spawn (lib.getExe pkgs.brightnessctl) "set" "10%+";
    "XF86MonBrightnessDown".action = spawn (lib.getExe pkgs.brightnessctl) "set" "10%-";

    # Audio Device Switching
    "Mod+F3".action = spawn "audio-switch" "toggle";

    # System
    "Mod+Shift+E".action = quit;
  };
}
