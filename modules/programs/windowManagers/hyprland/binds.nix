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
      inherit (import ./_helpers.nix { inherit lib; }) mkBinds;

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
      browser.run = "${lib.getExe config.homeModules.wlr-which-key.package} browser";
      screenshot.run = "${lib.getExe pkgs.chomp}";
      replay.run = "${lib.getExe pkgs.chomp} --replay save";
      fileManager = mkProg pkgs.nautilus;
      passwords = mkProg pkgs.proton-pass;
      sysMonitor = mkProg pkgs.resources;
      molt = mkProg osConfig.nixosModules.molt.package;
      hyprlock.run = "${lib.getExe config.programs.hyprlock.package} --grace 0";
      # shell.run = "${lib.getExe config.programs.noctalia.package} msg";
      shell.run = "${lib.getExe config.homeModules.island.package} ctl";
      mail.run = lib.getExe pkgs.protonmail-desktop;
      audioControl = mkProgWith pkgs.pwvucontrol "--tab 4";
      audioSwitch = lib.getExe osConfig.nixosModules.wireplumber.audioSwitching.package;
      volume =
        let
          wpctl = lib.getExe' pkgs.wireplumber "wpctl";
        in
        {
          up = "${wpctl} set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+";
          down = "${wpctl} set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%-";
          mute = "${wpctl} set-mute @DEFAULT_AUDIO_SINK@ toggle";
        };

      brightness =
        let
          brightnessctl = lib.getExe pkgs.brightnessctl;
        in
        {
          up = "${brightnessctl} set 5%+ -q";
          down = "${brightnessctl} set 5%- -n 5% -q";
        };

      media =
        let
          playerctl = lib.getExe pkgs.playerctl;
        in
        {
          play = "${playerctl} play-pause";
          prev = "${playerctl} previous";
          next = "${playerctl} next";
        };

      mainMod = "SUPER";
      mainModShift = "SUPER + SHIFT";
    in
    {
      wayland.windowManager.hyprland = {
        settings = {
          bind = mkBinds {
            # Applications
            "${mainMod} + ESCAPE" = {
              # rule = exec "${shell.run} panel-toggle session";
              rule = exec "${shell.run} toggle session";
              desc = "Session Menu";
            };
            "${mainMod} + L" = {
              rule = exec hyprlock.run;
              desc = "Lockscreen";
            };
            "${mainMod} + R" = {
              # rule = exec "${shell.run} panel-toggle launcher";
              rule = exec "${shell.run} toggle launcher";
              desc = "Application launcher";
            };
            "${mainMod} + N" = {
              # rule = exec "${shell.run} panel-toggle control-center notifications";
              rule = exec "${shell.run} toggle notifications";
              desc = "Notifications";
            };
            "${mainModShift} + N" = {
              # rule = exec "${shell.run} notification-clear-history";
              rule = exec "${shell.run} clear";
              desc = "Clear notifications";
            };
            "${mainMod} + D" = {
              rule = exec "${shell.run} toggle ai chat";
              desc = "AI chat";
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
              # rule = exec "${shell.run} panel-toggle clipboard";
              rule = exec "${shell.run} toggle clipboard";
              desc = "Clipboard Manager";
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
              rule = exec "hyprctl reload && hyprctl notify 5 2000 'rgb(89b4fa)' 'Config Reloaded'";
              desc = "Reload Hyprland config";
            };
            "${mainMod} + A" = {
              rule = exec "${audioSwitch} toggle";
              desc = "Toggle between audio devices";
            };
            "${mainMod} + U" = {
              rule = exec molt.run;
              desc = "Software Updates (Molt)";
            };
            "${mainMod} + S" = {
              rule = exec screenshot.run;
              desc = "Screenshot menu";
            };
            "${mainModShift} + S" = {
              rule = exec replay.run;
              desc = "Save instant replay";
            };

            # "${mainMod} + TAB" = {
            #   rule = mkInline ''hl.plugin.scrolloverview.overview("toggle")'';
            #   desc = "Toggle scroll overview";
            # };

            # Window management
            "${mainMod} + Q" = {
              rule = mkInline "hl.dsp.window.close()";
              desc = "Close active window";
            };
            "${mainModShift} + Q" = {
              rule = mkInline "hl.dsp.window.kill()";
              desc = "Force close active window";
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
              rule = exec volume.mute;
              desc = "Toggle mute";
              locked = true;
            };
            "XF86AudioPlay" = {
              rule = exec media.play;
              desc = "Play/pause media";
              locked = true;
            };
            "XF86AudioPrev" = {
              rule = exec media.prev;
              desc = "Previous track";
              locked = true;
            };
            "XF86AudioNext" = {
              rule = exec media.next;
              desc = "Next track";
              locked = true;
            };

            # Repeating keys (locked + repeat)
            "XF86MonBrightnessUp" = {
              rule = exec brightness.up;
              desc = "Increase brightness";
              locked = true;
              repeating = true;
            };
            "XF86MonBrightnessDown" = {
              rule = exec brightness.down;
              desc = "Decrease brightness";
              locked = true;
              repeating = true;
            };
            "XF86AudioRaiseVolume" = {
              rule = exec volume.up;
              desc = "Increase volume";
              locked = true;
              repeating = true;
            };
            "XF86AudioLowerVolume" = {
              rule = exec volume.down;
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
