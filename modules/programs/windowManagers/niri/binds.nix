{
  flake.modules.homeManager.niri =
    {
      pkgs,
      config,
      lib,
      osConfig,
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
          float = "${base} --class=floatTerm";
        };

      browser.run = "${lib.getExe config.homeModules.wlr-which-key.package} browser";
      screenshot.run = "${lib.getExe pkgs.chomp}";
      fileManager = mkProg pkgs.nautilus;
      passwords = mkProg pkgs.proton-pass;
      sysMonitor = mkProg pkgs.resources;
      swix = mkProg pkgs.swix;
      hyprlock.run = "${lib.getExe config.programs.hyprlock.package} --grace 0";
      shell.run = "${lib.getExe config.programs.noctalia.package} msg";
      mail.run = lib.getExe pkgs.protonmail-desktop;
      audioControl = mkProgWith pkgs.pwvucontrol "--tab 4";
      volume_script = lib.getExe pkgs.volume_script_hyprpanel;
      brightness_script = lib.getExe pkgs.brightness_script_hyprpanel;
      audioSwitch = lib.getExe osConfig.nixosModules.wireplumber.audioSwitching.package;
    in
    {
      wayland.windowManager.niri = {
        settings = {
          binds = {
            # Applications
            "Mod+Escape" = {
              _props.hotkey-overlay-title = "Session Menu";
              spawn-sh = "${shell.run} panel-toggle session";
            };
            "Mod+L" = {
              _props.hotkey-overlay-title = "Lockscreen";
              spawn-sh = hyprlock.run;
            };
            "Mod+R" = {
              _props.hotkey-overlay-title = "Application launcher";
              spawn-sh = "${shell.run} panel-toggle launcher";
            };
            "Mod+N" = {
              _props.hotkey-overlay-title = "Notifications";
              spawn-sh = "${shell.run} panel-toggle control-center notifications";
            };
            "Mod+Shift+N" = {
              _props.hotkey-overlay-title = "Clear notifications";
              spawn-sh = "${shell.run} notification-clear-history";
            };
            "Mod+B" = {
              _props.hotkey-overlay-title = "Browser";
              spawn-sh = browser.run;
            };
            "Mod+E" = {
              _props.hotkey-overlay-title = "File Manager";
              spawn-sh = fileManager.run;
            };
            "Mod+P" = {
              _props.hotkey-overlay-title = "Passwords";
              spawn-sh = passwords.run;
            };
            "Mod+Return" = {
              _props.hotkey-overlay-title = "Terminal";
              spawn-sh = term.run;
            };
            "Mod+Shift+Return" = {
              _props.hotkey-overlay-title = "Terminal (float)";
              spawn-sh = term.float;
            };
            "Ctrl+Shift+Escape" = {
              _props.hotkey-overlay-title = "System Monitor";
              spawn-sh = sysMonitor.run;
            };
            "Mod+V" = {
              _props.hotkey-overlay-title = "Clipboard Manager";
              spawn-sh = "${shell.run} panel-toggle clipboard";
            };
            "Mod+K" = {
              _props.hotkey-overlay-title = "Show keybinds";
              show-hotkey-overlay = { };
            };
            "Mod+G" = {
              _props.hotkey-overlay-title = "Audio Control";
              spawn-sh = audioControl.run;
            };
            "Mod+M" = {
              _props.hotkey-overlay-title = "Mail Client";
              spawn-sh = mail.run;
            };
            "Mod+A" = {
              _props.hotkey-overlay-title = "Toggle between audio devices";
              spawn-sh = "${audioSwitch} toggle";
            };
            "Mod+U" = {
              _props.hotkey-overlay-title = "Software Updates (Swix)";
              spawn-sh = swix.run;
            };
            "Mod+S" = {
              _props.hotkey-overlay-title = "Screenshot menu";
              spawn-sh = screenshot.run;
            };
            "Mod+Shift+R" = {
              _props.hotkey-overlay-title = "Reload config";
              spawn-sh = "notify-send -t 2000 'Niri' 'Config reloads automatically on save'";
            };

            # Screenshots
            "Print" = {
              _props.hotkey-overlay-title = "Screenshot interactive";
              screenshot = { };
            };
            "Ctrl+Print" = {
              _props.hotkey-overlay-title = "Screenshot screen";
              screenshot-screen = { };
            };
            "Alt+Print" = {
              _props.hotkey-overlay-title = "Screenshot window";
              screenshot-window = { };
            };

            # Window management
            "Mod+Q" = {
              _props.repeat = false;
              close-window = { };
            };
            "Mod+Shift+Q" = {
              _props.repeat = false;
              close-window = { };
            };
            "Mod+C" = {
              toggle-window-floating = { };
            };
            "Mod+F" = {
              maximize-column = { };
            };
            "Mod+Shift+F" = {
              fullscreen-window = { };
            };
            "Mod+O" = {
              _props.repeat = false;
              toggle-overview = { };
            };
            "Mod+J" = {
              consume-window-into-column = { };
            };
            "Mod+Shift+J" = {
              expel-window-from-column = { };
            };
            "Mod+W" = {
              toggle-column-tabbed-display = { };
            };
            "Mod+Shift+W" = {
              toggle-window-floating = { };
            };

            # Niri compositor actions
            "Mod+Ctrl+Shift+E" = {
              _props.repeat = false;
              _props.hotkey-overlay-title = "Force quit niri";
              quit = { };
            };
            "Mod+Shift+P" = {
              _props.repeat = false;
              _props.hotkey-overlay-title = "Power off monitors";
              power-off-monitors = { };
            };
            "Mod+Shift+I" = {
              _props.repeat = false;
              _props.hotkey-overlay-title = "Toggle shortcuts inhibit";
              toggle-keyboard-shortcuts-inhibit = { };
            };
            "Mod+T" = {
              _props.hotkey-overlay-title = "Toggle opacity";
              toggle-window-rule-opacity = { };
            };
            "Mod+D" = {
              _props.hotkey-overlay-title = "Center column";
              center-column = { };
            };
            "Mod+Shift+D" = {
              _props.hotkey-overlay-title = "Switch preset column width";
              switch-preset-column-width = { };
            };
            "Mod+Shift+H" = {
              _props.hotkey-overlay-title = "Reset window height";
              reset-window-height = { };
            };

            # Focus movement
            "Mod+Left".focus-column-left = { };
            "Mod+Right".focus-column-right = { };
            "Mod+Up".focus-window-up = { };
            "Mod+Down".focus-window-down = { };

            # Move windows / columns
            "Mod+Shift+Left".move-column-left = { };
            "Mod+Shift+Right".move-column-right = { };
            "Mod+Shift+Up".move-window-up = { };
            "Mod+Shift+Down".move-window-down = { };

            # Monitor focus and movement
            "Mod+Ctrl+Left".focus-monitor-left = { };
            "Mod+Ctrl+Right".focus-monitor-right = { };
            "Mod+Ctrl+Up".focus-monitor-up = { };
            "Mod+Ctrl+Down".focus-monitor-down = { };
            "Mod+Ctrl+Shift+Left".move-column-to-monitor-left = { };
            "Mod+Ctrl+Shift+Right".move-column-to-monitor-right = { };
            "Mod+Ctrl+Shift+Up".move-column-to-monitor-up = { };
            "Mod+Ctrl+Shift+Down".move-column-to-monitor-down = { };

            # Workspaces focus
            "Mod+1".focus-workspace = 1;
            "Mod+2".focus-workspace = 2;
            "Mod+3".focus-workspace = 3;
            "Mod+4".focus-workspace = 4;
            "Mod+5".focus-workspace = 5;
            "Mod+6".focus-workspace = 6;
            "Mod+7".focus-workspace = 7;
            "Mod+8".focus-workspace = 8;
            "Mod+9".focus-workspace = 9;
            "Mod+0".focus-workspace = 10;

            # Workspaces move
            "Mod+Shift+1".move-column-to-workspace = 1;
            "Mod+Shift+2".move-column-to-workspace = 2;
            "Mod+Shift+3".move-column-to-workspace = 3;
            "Mod+Shift+4".move-column-to-workspace = 4;
            "Mod+Shift+5".move-column-to-workspace = 5;
            "Mod+Shift+6".move-column-to-workspace = 6;
            "Mod+Shift+7".move-column-to-workspace = 7;
            "Mod+Shift+8".move-column-to-workspace = 8;
            "Mod+Shift+9".move-column-to-workspace = 9;
            "Mod+Shift+0".move-column-to-workspace = 10;

            # Sizing and column management
            "Mod+Minus".set-column-width = "-10%";
            "Mod+Equal".set-column-width = "+10%";
            "Mod+Shift+Minus".set-window-height = "-10%";
            "Mod+Shift+Equal".set-window-height = "+10%";
            "Mod+BracketLeft".consume-or-expel-window-left = { };
            "Mod+BracketRight".consume-or-expel-window-right = { };
            "Mod+Comma".consume-window-into-column = { };
            "Mod+Period".expel-window-from-column = { };
            "Mod+Alt+Up".move-workspace-up = { };
            "Mod+Alt+Down".move-workspace-down = { };
            "Mod+Home".focus-column-first = { };
            "Mod+End".focus-column-last = { };
            "Mod+Shift+Home".move-column-to-first = { };
            "Mod+Shift+End".move-column-to-last = { };
            "Mod+Tab" = {
              _props.repeat = false;
              focus-workspace-previous = { };
            };

            # Media keys (locked)
            "XF86AudioMute" = {
              _props.allow-when-locked = true;
              spawn-sh = "${volume_script} mute";
            };
            "XF86AudioPlay" = {
              _props.allow-when-locked = true;
              spawn-sh = "${lib.getExe pkgs.playerctl} play-pause";
            };
            "XF86AudioPrev" = {
              _props.allow-when-locked = true;
              spawn-sh = "${lib.getExe pkgs.playerctl} previous";
            };
            "XF86AudioNext" = {
              _props.allow-when-locked = true;
              spawn-sh = "${lib.getExe pkgs.playerctl} next";
            };

            # Brightness and Volume adjustment (locked)
            "XF86MonBrightnessUp" = {
              _props.allow-when-locked = true;
              spawn-sh = "${brightness_script} up";
            };
            "XF86MonBrightnessDown" = {
              _props.allow-when-locked = true;
              spawn-sh = "${brightness_script} down";
            };
            "XF86AudioRaiseVolume" = {
              _props.allow-when-locked = true;
              spawn-sh = "${volume_script} up";
            };
            "XF86AudioLowerVolume" = {
              _props.allow-when-locked = true;
              spawn-sh = "${volume_script} down";
            };

            # Mouse wheel scroll workspace
            "Mod+WheelScrollDown" = {
              _props.cooldown-ms = 150;
              focus-workspace-down = { };
            };
            "Mod+WheelScrollUp" = {
              _props.cooldown-ms = 150;
              focus-workspace-up = { };
            };
          };
        };
      };
    };
}
