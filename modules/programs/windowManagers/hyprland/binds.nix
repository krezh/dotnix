{
  flake.modules.homeManager.hyprland =
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
          float = cmd: "${base} --class=floatTerm -e ${cmd}";
          toggle = proc: cmd: "pkill ${proc} || ${base} --class=floatTerm -e ${cmd}";
        };

      mkInline = lib.generators.mkLuaInline;
      exec = cmd: mkInline "hl.dsp.exec_cmd(${builtins.toJSON cmd})";
      mkBinds = lib.mapAttrsToList (
        keys:
        { rule, ... }@opts:
        {
          _args = [
            keys
            rule
            (removeAttrs opts [ "rule" ])
          ];
        }
      );

      browser.run = "${lib.getExe config.programs.wlr-which-key.package} browser";
      screenshot.run = "${lib.getExe pkgs.chomp}";
      fileManager = mkProg pkgs.nautilus;
      passwords = mkProg pkgs.proton-pass;
      sysMonitor = mkProg pkgs.resources;
      hyprlock.run = "${lib.getExe config.programs.hyprlock.package} --grace 0";
      # launcher.run = "${pkgs.netcat}/bin/nc -U /run/user/$EUID/walker/walker.sock";
      shell.run = "${lib.getExe config.programs.noctalia.package} msg";
      keybinds.run = lib.getExe pkgs.hyprland_keybinds;
      # clipboardMgr.run = "${lib.getExe config.programs.walker.package} -m clipboard";
      mail.run = lib.getExe pkgs.protonmail-desktop;
      audioControl = mkProgWith pkgs.pwvucontrol "--tab 4";
      volume_script = lib.getExe pkgs.volume_script_hyprpanel;
      brightness_script = lib.getExe pkgs.brightness_script_hyprpanel;
      audioSwitch = lib.getExe osConfig.nixosModules.wireplumber.audioSwitching.package;

      mainMod = "SUPER";
      mainModShift = "SUPER + SHIFT";
    in
    {
      wayland.windowManager.hyprland = {
        settings = {
          bind = mkBinds {
            # Applications
            "${mainMod} + ESCAPE" = {
              rule = exec "${shell.run} panel-toggle session";
              desc = "Session Menu";
            };
            "${mainMod} + L" = {
              rule = exec hyprlock.run;
              desc = "Lockscreen";
            };
            "${mainMod} + R" = {
              rule = exec "${shell.run} panel-toggle launcher";
              desc = "Application launcher";
            };
            "${mainMod} + N" = {
              rule = exec "${shell.run} panel-toggle control-center notifications";
              desc = "Notifications";
            };
            "${mainModShift} + N" = {
              rule = exec "${shell.run} notification-clear-history";
              desc = "Clear notifications";
            };
            "${mainMod} + B" = {
              rule = exec browser.run;
              desc = "Browser";
            };
            "${mainMod} + E" = {
              rule = exec fileManager.run;
              desc = "File Manager";
            };
            "${mainMod} + P" = {
              rule = exec passwords.run;
              desc = "Passwords";
            };
            "${mainMod} + RETURN" = {
              rule = exec term.run;
              desc = "Terminal";
            };
            "${mainModShift} + RETURN" = {
              rule = exec "[float] ${term.run}";
              desc = "Terminal (float)";
            };
            "CTRL + SHIFT + ESCAPE" = {
              rule = exec "[float] ${sysMonitor.run}";
              desc = "System Monitor";
            };
            "${mainMod} + V" = {
              rule = exec "${shell.run} panel-toggle clipboard";
              desc = "Clipboard Manager";
            };
            "${mainMod} + K" = {
              rule = exec keybinds.run;
              desc = "Show keybinds";
            };
            "${mainMod} + G" = {
              rule = exec "[float] ${audioControl.run}";
              desc = "Audio Control";
            };
            "${mainMod} + M" = {
              rule = exec mail.run;
              desc = "Mail Client";
            };
            "${mainModShift} + R" = {
              rule = exec "hyprctl reload && notify-send --transient -u low 'Hyprland' 'Config Reloaded'";
              desc = "Reload Hyprland config";
            };
            "${mainMod} + A" = {
              rule = exec "${audioSwitch} toggle";
              desc = "Toggle between audio devices";
            };
            "${mainMod} + S" = {
              rule = exec screenshot.run;
              desc = "Screenshot menu";
            };

            # Window management
            "${mainMod} + Q" = {
              rule = mkInline "hl.dsp.window.close()";
              desc = "Close active window";
            };
            "${mainMod} + C" = {
              rule = mkInline "hl.dsp.window.float()";
              desc = "Toggle floating mode";
            };
            "${mainMod} + J" = {
              rule = mkInline ''hl.dsp.layout("togglesplit")'';
              desc = "Toggle split layout";
            };
            "${mainMod} + F" = {
              rule = mkInline ''hl.dsp.window.fullscreen({mode = "maximized", action = "toggle"})'';
              desc = "Maximize window";
            };
            "${mainModShift} + F" = {
              rule = mkInline ''hl.dsp.window.fullscreen({mode = "fullscreen", action = "toggle"})'';
              desc = "Toggle fullscreen";
            };

            # Move windows
            "${mainModShift} + LEFT" = {
              rule = mkInline ''hl.dsp.window.move({direction = "left"})'';
              desc = "Move window left";
            };
            "${mainModShift} + RIGHT" = {
              rule = mkInline ''hl.dsp.window.move({direction = "right"})'';
              desc = "Move window right";
            };
            "${mainModShift} + UP" = {
              rule = mkInline ''hl.dsp.window.move({direction = "up"})'';
              desc = "Move window up";
            };
            "${mainModShift} + DOWN" = {
              rule = mkInline ''hl.dsp.window.move({direction = "down"})'';
              desc = "Move window down";
            };

            # Move focus
            "${mainMod} + left" = {
              rule = mkInline ''hl.dsp.focus({direction = "left"})'';
              desc = "Move focus left";
            };
            "${mainMod} + right" = {
              rule = mkInline ''hl.dsp.focus({direction = "right"})'';
              desc = "Move focus right";
            };
            "${mainMod} + up" = {
              rule = mkInline ''hl.dsp.focus({direction = "up"})'';
              desc = "Move focus up";
            };
            "${mainMod} + down" = {
              rule = mkInline ''hl.dsp.focus({direction = "down"})'';
              desc = "Move focus down";
            };

            # Switch workspaces
            "${mainMod} + 1" = {
              rule = mkInline "hl.dsp.focus({workspace = 1})";
              desc = "Switch to workspace 1";
            };
            "${mainMod} + 2" = {
              rule = mkInline "hl.dsp.focus({workspace = 2})";
              desc = "Switch to workspace 2";
            };
            "${mainMod} + 3" = {
              rule = mkInline "hl.dsp.focus({workspace = 3})";
              desc = "Switch to workspace 3";
            };
            "${mainMod} + 4" = {
              rule = mkInline "hl.dsp.focus({workspace = 4})";
              desc = "Switch to workspace 4";
            };
            "${mainMod} + 5" = {
              rule = mkInline "hl.dsp.focus({workspace = 5})";
              desc = "Switch to workspace 5";
            };
            "${mainMod} + 6" = {
              rule = mkInline "hl.dsp.focus({workspace = 6})";
              desc = "Switch to workspace 6";
            };
            "${mainMod} + 7" = {
              rule = mkInline "hl.dsp.focus({workspace = 7})";
              desc = "Switch to workspace 7";
            };
            "${mainMod} + 8" = {
              rule = mkInline "hl.dsp.focus({workspace = 8})";
              desc = "Switch to workspace 8";
            };
            "${mainMod} + 9" = {
              rule = mkInline "hl.dsp.focus({workspace = 9})";
              desc = "Switch to workspace 9";
            };
            "${mainMod} + 0" = {
              rule = mkInline "hl.dsp.focus({workspace = 10})";
              desc = "Switch to workspace 10";
            };

            # Special workspace
            "${mainMod} + W" = {
              rule = mkInline "hl.dsp.workspace.toggle_special()";
              desc = "Toggle special workspace";
            };
            "${mainModShift} + W" = {
              rule = mkInline ''hl.dsp.window.move({workspace = "special"})'';
              desc = "Move window to special workspace";
            };

            # Move windows to workspaces
            "${mainModShift} + 1" = {
              rule = mkInline "hl.dsp.window.move({workspace = 1})";
              desc = "Move active window to workspace 1";
            };
            "${mainModShift} + 2" = {
              rule = mkInline "hl.dsp.window.move({workspace = 2})";
              desc = "Move active window to workspace 2";
            };
            "${mainModShift} + 3" = {
              rule = mkInline "hl.dsp.window.move({workspace = 3})";
              desc = "Move active window to workspace 3";
            };
            "${mainModShift} + 4" = {
              rule = mkInline "hl.dsp.window.move({workspace = 4})";
              desc = "Move active window to workspace 4";
            };
            "${mainModShift} + 5" = {
              rule = mkInline "hl.dsp.window.move({workspace = 5})";
              desc = "Move active window to workspace 5";
            };
            "${mainModShift} + 6" = {
              rule = mkInline "hl.dsp.window.move({workspace = 6})";
              desc = "Move active window to workspace 6";
            };
            "${mainModShift} + 7" = {
              rule = mkInline "hl.dsp.window.move({workspace = 7})";
              desc = "Move active window to workspace 7";
            };
            "${mainModShift} + 8" = {
              rule = mkInline "hl.dsp.window.move({workspace = 8})";
              desc = "Move active window to workspace 8";
            };
            "${mainModShift} + 9" = {
              rule = mkInline "hl.dsp.window.move({workspace = 9})";
              desc = "Move active window to workspace 9";
            };
            "${mainModShift} + 0" = {
              rule = mkInline "hl.dsp.window.move({workspace = 10})";
              desc = "Move active window to workspace 10";
            };

            # Media keys (locked)
            "XF86AudioMute" = {
              rule = exec "${volume_script} mute";
              desc = "Toggle mute";
              locked = true;
            };
            "XF86AudioPlay" = {
              rule = exec "${lib.getExe pkgs.playerctl} play-pause";
              desc = "Play/pause media";
              locked = true;
            };
            "XF86AudioPrev" = {
              rule = exec "${lib.getExe pkgs.playerctl} previous";
              desc = "Previous track";
              locked = true;
            };
            "XF86AudioNext" = {
              rule = exec "${lib.getExe pkgs.playerctl} next";
              desc = "Next track";
              locked = true;
            };

            # Repeating keys (locked + repeat)
            "XF86MonBrightnessUp" = {
              rule = exec "${brightness_script} up";
              desc = "Increase brightness";
              locked = true;
              repeating = true;
            };
            "XF86MonBrightnessDown" = {
              rule = exec "${brightness_script} down";
              desc = "Decrease brightness";
              locked = true;
              repeating = true;
            };
            "XF86AudioRaiseVolume" = {
              rule = exec "${volume_script} up";
              desc = "Increase volume";
              locked = true;
              repeating = true;
            };
            "XF86AudioLowerVolume" = {
              rule = exec "${volume_script} down";
              desc = "Decrease volume";
              locked = true;
              repeating = true;
            };

            # Mouse binds
            "${mainMod} + mouse:272" = {
              rule = mkInline "hl.dsp.window.drag()";
              desc = "Move window";
              mouse = true;
            };
            "${mainMod} + mouse:273" = {
              rule = mkInline "hl.dsp.window.resize()";
              desc = "Resize window";
              mouse = true;
            };
            "${mainModShift} + mouse:272" = {
              rule = mkInline "hl.dsp.window.resize()";
              desc = "Resize window";
              mouse = true;
            };
          };
        };
      };
    };
}
