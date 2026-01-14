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
          base = lib.getExe pkgs.kitty;
        in
        {
          run = "${base}";
          float = cmd: "${base} --class floatTerm -e ${cmd}";
          toggle = proc: cmd: "pkill ${proc} || ${base} --class floatTerm -e ${cmd}";
        };

      browser = mkProg config.programs.zen-browser.package;
      fileManager = mkProg pkgs.nautilus;
      passwords = mkProg pkgs.bww;
      sysMonitor = mkProg pkgs.mission-center;
      logout = mkProg pkgs.wlogout;
      hyprlock.run = "${lib.getExe config.programs.hyprlock.package} --immediate";
      launcher.run = "${pkgs.netcat}/bin/nc -U /run/user/$(id -u)/walker/walker.sock";
      shell.run = lib.getExe config.programs.noctalia-shell.package;
      keybinds.run = lib.getExe pkgs.hyprland_keybinds;
      clipboardMgr.run = "${lib.getExe config.programs.walker.package} -m clipboard";
      mail.run = lib.getExe' pkgs.geary "geary";
      audioControl = mkProgWith pkgs.wiremix "-m 100";
      trayTui = mkProg pkgs.tray-tui;
      volume_script = lib.getExe pkgs.volume_script_hyprpanel;
      brightness_script = lib.getExe pkgs.brightness_script_hyprpanel;

      recShot = "${lib.getExe pkgs.recshot} -t ${
        config.sops.secrets."zipline/token".path
      } -u https://zipline.talos.plexuz.xyz";

      mainMod = "SUPER";
      mainModShift = "${mainMod} SHIFT";
    in
    {
      services.polkit-gnome.enable = true;

      wayland.windowManager.hyprland = {
        enable = true;
        xwayland.enable = true;
        systemd = {
          enable = true;
          enableXdgAutostart = true;
          variables = [ "--all" ];
        };
        plugins = with pkgs.hyprlandPlugins; [
          hyprexpo
        ];

        settings = {
          "$mainMod" = "${mainMod}";

          env = [
            "QT_WAYLAND_DISABLE_WINDOWDECORATION,1"
            "QT_QPA_PLATFORM=wayland"
          ];

          workspace = [
            "1,monitor:DP-1"
            "2,monitor:DP-1"
            "3,monitor:DP-1"
            "4,monitor:DP-2"
            "5,monitor:DP-2"
            "6,monitor:DP-2"
            # Smart gaps
            "w[tv1]s[false], gapsout:0, gapsin:0"
            "f[1]s[false], gapsout:0, gapsin:0"
          ];

          monitorv2 =
            if osConfig.networking.hostName == "thor" then
              [
                {
                  output = "DP-1";
                  mode = "2560x1440@239.97";
                  position = "0x0";
                  scale = 1.0;
                  bitdepth = 10;
                  vrr = 2;
                }
                {
                  output = "DP-2";
                  mode = "2560x1440@144";
                  position = "2560x0";
                  scale = 1.0;
                }
              ]
            else if osConfig.networking.hostName == "odin" then
              [
                {
                  output = "eDP-1";
                  mode = "1920x1080@60.0";
                  position = "0x0";
                  scale = 1;
                }
              ]
            else if osConfig.networking.hostName == "steamdeck" then
              [
                {
                  output = "eDP-1";
                  mode = "800x1280@90";
                  position = "0x0";
                  transform = 3;
                  scale = 1;
                }
              ]
            else
              [
                {
                  output = "";
                  mode = "preferred";
                  position = "auto";
                  scale = 1;
                }
              ];

          xwayland.force_zero_scaling = true;

          cursor = {
            enable_hyprcursor = true;
            no_warps = true;
          };

          gestures.workspace_swipe_cancel_ratio = 0.15;

          input = {
            kb_layout = "se";
            kb_variant = "nodeadkeys";
            follow_mouse = 2;
            float_switch_override_focus = 0;
            accel_profile = "flat";
            numlock_by_default = true;
            touchpad.natural_scroll = false;
            sensitivity = "0.4";
          };

          plugin.hyprexpo = {
            columns = 2;
            gap_size = 5;
            bg_col = "rgb(111111)";
            workspace_method = "center current";
            gesture_distance = 300;
            skip_empty = true;
          };

          general = {
            layout = "dwindle";
            gaps_in = 5;
            gaps_out = 10;
            border_size = 3;
            "col.active_border" = "$blue $green 125deg";
            "col.inactive_border" = "$base";
            allow_tearing = false;
          };

          decoration = {
            rounding = config.var.rounding;
            rounding_power = 4;
            active_opacity = config.var.opacity;
            inactive_opacity = config.var.opacity;
            blur = {
              enabled = false;
              passes = 4;
              size = 7;
              noise = 0.01;
              ignore_opacity = true;
              brightness = 1.0;
              contrast = 1.0;
              vibrancy = 0.8;
              vibrancy_darkness = 0.6;
              popups = true;
              popups_ignorealpha = 0.2;
            };
            shadow = {
              enabled = true;
              color = "rgba(00000055)";
              ignore_window = true;
              offset = "0 15";
              range = 100;
              render_power = 3;
              scale = 0.97;
            };
          };

          misc = {
            vfr = true;
            vrr = 2;
            enable_swallow = true;
            mouse_move_enables_dpms = true;
            key_press_enables_dpms = true;
            animate_manual_resizes = false;
            middle_click_paste = false;
            focus_on_activate = true;
            disable_hyprland_logo = true;
            disable_splash_rendering = true;
            disable_autoreload = true;
            session_lock_xray = true;
            new_window_takes_over_fullscreen = 2;
            render_unfocused_fps = 30;
          };

          dwindle = {
            force_split = 0;
            preserve_split = true;
            default_split_ratio = 1.0;
            special_scale_factor = 0.8;
            split_width_multiplier = 1.0;
            use_active_for_splits = true;
          };

          debug.damage_tracking = 2;

          animations = {
            enabled = true;
            bezier = [
              "easeOutExpo,0.16,1,0.3,1"
              "easeOutQuad,0.25,0.46,0.45,0.94"
              "spring,0.25,0.1,0.25,1"
            ];
            animation = [
              "windowsIn,1,3,spring"
              "windowsOut,1,2,easeOutQuad,popin 80%"
              "windowsMove,1,3,spring"
              "workspaces,1,3,spring,slidevert"
              "border,1,3,spring"
              "fade,1,1,spring"
              "layers,1,3,easeOutQuad"
            ];
          };

          layerrule = [
            "blur,rofi"
            "animation popin 80%, launcher"
            "blur, launcher"
            "animation popin 60%, walker"
            "animation fade, hyprpicker"
            "animation fade, logout_dialog"
            "animation fade, selection"
            "animation fade, wayfreeze"
            "noanim, (noctalia:.*)"
          ];

          windowrule = [
            # Smart gaps
            "bordersize 0, floating:0, onworkspace:w[tv1]s[false]"
            "rounding 0, floating:0, onworkspace:w[tv1]s[false]"
            "bordersize 0, floating:0, onworkspace:f[1]s[false]"
            "rounding 0, floating:0, onworkspace:f[1]s[false]"
            # Rofi
            "stayfocused, class:(Rofi)"
            # Chat workspace
            "workspace 4 silent, tag:chat"
            # Fullscreen opacity
            "opacity 1.0 override,fullscreen:1"
            # Games workspace and idle inhibit
            "workspace 3, tag:games"
            "idleinhibit always, tag:games"
            "idleinhibit fullscreen, fullscreen:1"
            # XWayland popups
            "nodim, xwayland:1, title:win[0-9]+"
            "noshadow, xwayland:1, title:win[0-9]+"
            "rounding ${toString config.var.rounding}, xwayland:1, title:win[0-9]+"
            # Dialog windows
            "float, title:(Select|Open)( a)? (File|Folder)(s)?"
            "float, title:File (Operation|Upload)( Progress)?"
            "float, title:.* Properties"
            "float, title:Export Image as PNG"
            "float, title:GIMP Crash Debug"
            "float, title:Save As"
            "float, title:Library"
            "float, title:Install, class:steam"
            "size 50% 50%, title:Install, class:steam"
            # File managers
            "float, class:org\\.gnome\\.FileRoller"
            "float, class:file-roller"
            # Vips image viewer
            "float, class:org\\.libvips\\.vipsdisp"
            # MPV
            "float, class:mpv"
            "size 60% 70%,class:mpv"
            # Float Terminal
            "float, class:floatTerm"
            "size 60% 60%, class:floatTerm"
            "float, class:com.example.floatterm"
            "size 60% 60%, class:com.example.floatterm"
            # Tag games
            "tag +games, class:^(gamescope)$"
            "tag +games, class:^(steam_proton)$"
            "tag +games, class:^(steam_app_default)$"
            "tag +games, class:^(steam_app_[0-9]+)$"
            # Tag browsers
            "tag +browsers, class:^(zen.*)$"
            "tag +browsers, class:^(firefox)$"
            "tag +browsers, class:^(chromium)$"
            "tag +browsers, class:^(chrome)$"
            "tag +browsers, class:^(vivaldi)$"
            # Tag media
            "tag +media, class:^(mpv)$"
            "tag +media, class:^(vlc)$"
            "tag +media, class:^(youtube)$"
            "tag +media, class:^(plex)$"
            "tag +media, class:^(org.jellyfin.JellyfinDesktop)$"
            # Tag chat
            "tag +chat, class:^(vesktop)$"
            "tag +chat, class:^(legcord)$"
            "tag +chat, class:^(discord)$"
            # Opacity overrides
            "opacity 1.0 override,tag:games"
            "opacity 1.0 override,tag:browsers"
            "opacity 1.0 override,tag:media"
            "opacity 1.0 override,initialTitle:^(Discord Popout)$"
            # Blur exceptions
            "noblur,tag:games"
            "noblur,tag:media"
            # Render unfocused
            "renderunfocused,tag:games"
          ];

          bindd = [
            "${mainMod},ESCAPE,Logout Menu,exec,${logout.run}"
            "${mainMod},L,Lockscreen,exec,${hyprlock.run}"
            "${mainMod},R,Application launcher,exec,${launcher.run}"
            "${mainMod},N,Notifications,exec,${shell.run} ipc call notifications toggleHistory"
            "${mainModShift},N,Clear notifications,exec,${shell.run} ipc call notifications clear"
            "${mainMod},B,Browser,exec,${browser.run}"
            "${mainMod},E,File Manager,exec,${fileManager.run}"
            "${mainModShift},E,Floating File Manager,exec,[float] ${fileManager.run}"
            "${mainMod},P,Passwords,exec,${passwords.run}"
            "${mainMod},RETURN,Terminal,exec,${term.run}"
            "${mainModShift},RETURN,Terminal,exec,[float] ${term.run}"
            "${mainMod},T,Tray-Tui,exec,[float] ${term.toggle trayTui.name trayTui.run}"
            "CTRL SHIFT,ESCAPE,System Monitor,exec,[float] ${sysMonitor.run}"
            "${mainMod},V,Clipboard Manager,exec,${clipboardMgr.run}"
            "${mainMod},K,Show keybinds,exec,${keybinds.run}"
            "${mainMod},G,Audio Control,exec,[float] ${term.toggle audioControl.name audioControl.run}"
            "${mainMod},M,Mail Client,exec,${mail.run}"
            "${mainMod},TAB,Toggle workspace overview, hyprexpo:expo, toggle"
            "${mainMod},F3,Toggle between audio devices,exec,audio-switch toggle"
            "${mainModShift},S,Area screenshot,exec,${recShot} -m image-area"
            ",PRINT,Fullscreen screenshot,exec,${recShot} -m image-full"
            "ALT,PRINT,Window screenshot,exec,${recShot} -m image-window"
            "SHIFT ALT,S,Area screen recording,exec,${recShot} -m video-area"
            "SHIFT,PRINT,Window screen recording,exec,${recShot} -m video-window"
            "${mainModShift},C,Gulp OCR,exec,${lib.getExe pkgs.gulp} --ocr --no-snap"
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
            "${mainMod},mouse_down,Next workspace,workspace,e+1"
            "${mainMod},mouse_up,Previous workspace,workspace,e-1"
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

      # XDPH config
      xdg.configFile."hypr/xdph.conf".text = ''
        screencopy {
            max_fps = 120
            allow_token_by_default = true;
        }
      '';
      home.packages = [
        pkgs.tray-tui
        pkgs.hyprpicker
        pkgs.hyprmon
        pkgs.hyprshade
        pkgs.hyprdynamicmonitors
        # Exit script
        (pkgs.writeShellScriptBin "hyprexit" ''
          ${pkgs.hyprland}/bin/hyprctl dispatch exit
          ${pkgs.systemd}/bin/loginctl terminate-user ${config.home.username}
        '')
      ];
    };
}
