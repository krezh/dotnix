{
  flake.modules.homeManager.umbriel =
    {
      pkgs,
      config,
      lib,
      osConfig,
      ...
    }:
    let
      mkProg = pkg: lib.getExe pkg;
      spawn = command: "spawn:${command}";
      noRepeat = action: {
        inherit action;
        repeat = false;
      };
      locked = action: {
        inherit action;
        allow_when_locked = true;
      };

      term = lib.getExe pkgs.ghostty;
      browser = "${lib.getExe config.homeModules.wlr-which-key.package} browser";
      screenshot = lib.getExe pkgs.chomp;
      fileManager = mkProg pkgs.nautilus;
      passwords = mkProg pkgs.proton-pass;
      sysMonitor = mkProg pkgs.resources;
      molt = mkProg osConfig.nixosModules.molt.package;
      hyprlock = "${lib.getExe config.programs.hyprlock.package} --grace 0";
      shell = "${lib.getExe config.programs.noctalia.package} msg";
      mail = mkProg pkgs.protonmail-desktop;
      audioControl = "${lib.getExe pkgs.pwvucontrol} --tab 4";
      audioSwitch = lib.getExe osConfig.nixosModules.wireplumber.audioSwitching.package;

      isThor = osConfig.networking.hostName == "thor";
      workspaceSelector =
        index:
        if isThor && index <= 6 then
          ''"${toString index}"/${if index <= 3 then "DP-1" else "DP-2"}''
        else
          toString index;
      workspaceKey = index: if index == 10 then "0" else toString index;
      workspaceBinds = lib.listToAttrs (
        map (index: {
          name = "Mod+${workspaceKey index}";
          value = "workspace-switch:${workspaceSelector index}";
        }) (lib.range 1 10)
      );
      moveWorkspaceBinds = lib.listToAttrs (
        map (index: {
          name = "Mod+Shift+${workspaceKey index}";
          value = "column-move-to-workspace:${workspaceSelector index}";
        }) (lib.range 1 10)
      );
    in
    {
      programs.umbriel.settings.keybinds = {
        "Mod+Escape" = spawn "${shell} panel-toggle session";
        "Mod+L" = spawn hyprlock;
        "Mod+R" = spawn "${shell} panel-toggle launcher";
        "Mod+N" = spawn "${shell} panel-toggle control-center notifications";
        "Mod+Shift+N" = spawn "${shell} notification-clear-history";
        "Mod+B" = spawn browser;
        "Mod+E" = spawn fileManager;
        "Mod+P" = spawn passwords;
        "Mod+Return" = spawn "${term} +new-window";
        "Mod+Shift+Return" = spawn "${term} --class=floatTerm";
        "Ctrl+Shift+Escape" = spawn sysMonitor;
        "Mod+V" = spawn "${shell} panel-toggle clipboard";
        "Mod+K" = "cheatsheet-toggle";
        "Mod+G" = spawn audioControl;
        "Mod+M" = spawn mail;
        "Mod+A" = spawn "${audioSwitch} toggle";
        "Mod+U" = spawn molt;
        "Mod+S" = spawn screenshot;
        "Mod+Shift+R" = "config-reload";

        "Print" = spawn screenshot;

        "Mod+Q" = noRepeat "window-close";
        "Mod+Shift+Q" = noRepeat "window-close";
        "Mod+C" = "window-toggle-floating";
        "Mod+F" = "window-toggle-maximize";
        "Mod+Shift+F" = "window-toggle-fullscreen";
        "Mod+O" = noRepeat "overview-toggle";
        "Mod+J" = "window-consume-left";
        "Mod+Shift+J" = "window-consume-or-expel-right";
        "Mod+W" = "column-toggle-tabbed";
        "Mod+Shift+W" = "window-toggle-floating";

        "Mod+Ctrl+Shift+E" = noRepeat "session-quit:skip-confirmation";
        "Mod+Shift+P" = noRepeat "dpms-off";
        "Mod+Shift+I" = {
          action = "shortcuts-inhibit-toggle";
          allow_when_inhibited = true;
          repeat = false;
        };
        "Mod+D" = "column-center";
        "Mod+Shift+D" = "window-cycle-primary-extent";
        "Mod+Shift+H" = "window-set-secondary-extent:1.0";

        "Mod+Left" = "window-focus-left";
        "Mod+Right" = "window-focus-right";
        "Mod+Up" = "window-focus-up";
        "Mod+Down" = "window-focus-down";

        "Mod+Shift+Left" = "column-move-left";
        "Mod+Shift+Right" = "column-move-right";
        "Mod+Shift+Up" = "window-move-up";
        "Mod+Shift+Down" = "window-move-down";

        "Mod+Ctrl+Left" = "output-focus-left";
        "Mod+Ctrl+Right" = "output-focus-right";
        "Mod+Ctrl+Up" = "output-focus-up";
        "Mod+Ctrl+Down" = "output-focus-down";
        "Mod+Ctrl+Shift+Left" = "column-move-to-output-left";
        "Mod+Ctrl+Shift+Right" = "column-move-to-output-right";
        "Mod+Ctrl+Shift+Up" = "column-move-to-output-up";
        "Mod+Ctrl+Shift+Down" = "column-move-to-output-down";

        "Mod+Minus" = "window-modify-primary-extent:-0.1";
        "Mod+Equal" = "window-modify-primary-extent:0.1";
        "Mod+Shift+Minus" = "window-modify-secondary-extent:-0.1";
        "Mod+Shift+Equal" = "window-modify-secondary-extent:0.1";
        "Mod+BracketLeft" = "window-consume-or-expel-left";
        "Mod+BracketRight" = "window-consume-or-expel-right";
        "Mod+Comma" = "window-consume-left";
        "Mod+Period" = "window-consume-right";
        "Mod+Alt+Up" = "workspace-move-up";
        "Mod+Alt+Down" = "workspace-move-down";
        "Mod+Home" = "column-focus-first";
        "Mod+End" = "column-focus-last";
        "Mod+Shift+Home" = "column-move-to-first";
        "Mod+Shift+End" = "column-move-to-last";
        "Mod+Tab" = noRepeat "workspace-focus-last";

        "XF86AudioPlay" = locked (spawn "${lib.getExe pkgs.playerctl} play-pause");
        "XF86AudioPrev" = locked (spawn "${lib.getExe pkgs.playerctl} previous");
        "XF86AudioNext" = locked (spawn "${lib.getExe pkgs.playerctl} next");

        "Mod+WheelDown" = {
          action = "workspace-next";
          cooldown_ms = 150;
        };
        "Mod+WheelUp" = {
          action = "workspace-previous";
          cooldown_ms = 150;
        };
      }
      // workspaceBinds
      // moveWorkspaceBinds;
    };
}
