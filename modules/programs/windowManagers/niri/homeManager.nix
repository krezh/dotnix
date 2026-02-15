{ inputs, ... }:
{
  flake.modules.homeManager.niri =
    {
      pkgs,
      lib,
      config,
      ...
    }:
    let
      termBin = lib.getExe pkgs.ghostty;
      launcherBin = "${pkgs.netcat}/bin/nc -U /run/user/$(id -u)/walker/walker.sock";
      shellBin = "${lib.getExe config.programs.noctalia-shell.package} ipc call";
      clipboardBin = "${lib.getExe config.programs.walker.package} -m clipboard";
      recShot = "${lib.getExe pkgs.recshot} -t ${
        config.sops.secrets."zipline/token".path
      } -u https://zipline.talos.plexuz.xyz";

      getBinaryName = pkg: pkg.meta.mainProgram or pkg.pname or pkg.name;
      audioControlPkg = pkgs.wiremix;
      audioControlBin = lib.getExe audioControlPkg;
      audioControlName = getBinaryName audioControlPkg;

      actions = config.lib.niri.actions;
    in
    {
      imports = [
        inputs.niri.homeModules.niri
      ];

      programs.niri.settings = {
        # General settings
        prefer-no-csd = true;
        clipboard.disable-primary = true;
        hotkey-overlay.skip-at-startup = true;
        gestures.hot-corners.enable = false;

        # XWayland
        xwayland-satellite = {
          enable = true;
          path = "${pkgs.xwayland-satellite}/bin/xwayland-satellite";
        };

        # Environment
        environment = {
          NIXOS_OZONE_WL = "1";
          MOZ_ENABLE_WAYLAND = "1";
          QT_QPA_PLATFORM = "wayland";
          QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
          SDL_VIDEODRIVER = "wayland";
          _JAVA_AWT_WM_NONREPARENTING = "1";
        };

        # Startup
        spawn-at-startup = [
          {
            argv = [
              "dbus-update-activation-environment"
              "--systemd"
              "--all"
            ];
          }
        ];

        # Input
        input = {
          keyboard = {
            xkb.layout = "se";
          };
          touchpad = {
            tap = true;
            dwt = true;
            dwtp = true;
            natural-scroll = true;
            accel-profile = "flat";
            accel-speed = 0.4;
          };
          mouse = {
            accel-profile = "flat";
            accel-speed = 0.4;
          };
        };

        # Outputs
        outputs = {
          "DP-1" = {
            mode = {
              width = 2560;
              height = 1440;
              refresh = 239.970;
            };
            position = {
              x = 0;
              y = 0;
            };
            scale = 1;
          };
          "DP-2" = {
            mode = {
              width = 2560;
              height = 1440;
              refresh = 143.998;
            };
            position = {
              x = 2560;
              y = 0;
            };
            scale = 1;
          };
        };

        # Workspaces
        workspaces = {
          "a-1" = {
            name = "1";
            open-on-output = "DP-1";
          };
          "a-2" = {
            name = "2";
            open-on-output = "DP-1";
          };
          "a-3" = {
            name = "3";
            open-on-output = "DP-1";
          };
          "a-4" = {
            name = "4";
            open-on-output = "DP-2";
          };
          "a-5" = {
            name = "5";
            open-on-output = "DP-2";
          };
          "a-6" = {
            name = "6";
            open-on-output = "DP-2";
          };
        };

        # Layout
        layout = {
          gaps = 10;
          focus-ring = {
            width = config.var.borderSize;
            active.color = "#89b4faff";
            inactive.color = "#1e1e2eff";
          };
          border.enable = false;
        };

        cursor = {
          theme = "catppuccin-mocha-dark-cursors";
          size = 24;
        };

        # Window rules
        window-rules = [
          {
            geometry-corner-radius = {
              bottom-left = 15.0;
              bottom-right = 15.0;
              top-left = 15.0;
              top-right = 15.0;
            };
            clip-to-geometry = true;
            draw-border-with-background = false;
            opacity = config.var.opacity;
            shadow = {
              enable = true;
            };
          }
          {
            matches = [
              {
                app-id = "steam";
                title = ''^notificationtoasts_\d+_desktop$'';
              }
            ];
            default-floating-position = {
              x = 10;
              y = 10;
              relative-to = "bottom-right";
            };
          }

          # Showkey
          {
            matches = [ { "app-id" = "^showkey$"; } ];
            "open-floating" = true;
          }

          # Audio control
          {
            matches = [ { "app-id" = "^audioControl$"; } ];
            "open-floating" = true;
          }

          # MPV
          {
            matches = [ { "app-id" = "^mpv$"; } ];
            "open-floating" = true;
            "default-column-width" = {
              proportion = 0.6;
            };
            opacity = 1.0;
          }

          # Browsers - full opacity
          {
            matches = [ { "app-id" = "^zen-beta$"; } ];
            opacity = 1.0;
          }

          {
            matches = [ { "app-id" = "^firefox$"; } ];
            opacity = 1.0;
          }

          {
            matches = [ { "app-id" = "^chromium$"; } ];
            opacity = 1.0;
          }

          {
            matches = [ { "app-id" = "^chrome$"; } ];
            opacity = 1.0;
          }

          # Chat applications
          {
            matches = [ { "app-id" = "^vesktop$"; } ];
            "open-on-workspace" = "4";
          }

          {
            matches = [ { "app-id" = "^discord$"; } ];
            "open-on-workspace" = "4";
          }

          {
            matches = [ { "app-id" = "^legcord$"; } ];
            "open-on-workspace" = "4";
          }

          # Games
          {
            matches = [ { "app-id" = "^gamescope$"; } ];
            "open-on-workspace" = "3";
            opacity = 1.0;
            "open-fullscreen" = true;
          }

          {
            matches = [ { "app-id" = "^steam_app_.*$"; } ];
            "open-on-workspace" = "3";
            opacity = 1.0;
            "open-fullscreen" = true;
          }

          {
            matches = [ { "app-id" = "^steam_proton$"; } ];
            "open-on-workspace" = "3";
            opacity = 1.0;
            "open-fullscreen" = true;
          }

          # VLC
          {
            matches = [ { "app-id" = "^vlc$"; } ];
            opacity = 1.0;
          }

          # Dialog windows
          {
            matches = [ { title = "^(Select|Open)( a)? (File|Folder)(s)?$"; } ];
            "open-floating" = true;
          }

          {
            matches = [ { title = "^File (Operation|Upload)( Progress)?$"; } ];
            "open-floating" = true;
          }

          {
            matches = [ { title = "^.* Properties$"; } ];
            "open-floating" = true;
          }

          {
            matches = [ { title = "^Save As$"; } ];
            "open-floating" = true;
          }

          {
            matches = [ { title = "^Library$"; } ];
            "open-floating" = true;
          }

          # GIMP dialogs
          {
            matches = [ { title = "^Export Image as PNG$"; } ];
            "open-floating" = true;
          }

          {
            matches = [ { title = "^GIMP Crash Debug$"; } ];
            "open-floating" = true;
          }

          # vipsdisp image viewer
          {
            matches = [ { "app-id" = "^org\\.libvips\\.vipsdisp$"; } ];
            "open-floating" = true;
          }

          # Media applications - full opacity
          {
            matches = [ { "app-id" = "^mpv$"; } ];
            opacity = 1.0;
          }

          {
            matches = [ { "app-id" = "^youtube$"; } ];
            opacity = 1.0;
          }

          {
            matches = [ { "app-id" = "^plex$"; } ];
            opacity = 1.0;
          }
        ];

        # Keybindings
        binds = with actions; {
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
          "Mod+Return".action = spawn termBin "+new-window";
          "Mod+R".action = spawn-sh launcherBin;
          "Mod+B".action = spawn (lib.getExe config.programs.zen-browser.package);
          "Mod+E".action = spawn (lib.getExe pkgs.nautilus);
          "Mod+O".action = spawn (lib.getExe pkgs.gnome-calculator);
          "Mod+V".action = spawn-sh clipboardBin;
          "Mod+N".action = spawn-sh "${shellBin} notifications toggleHistory";
          "Mod+Ctrl+N".action = spawn-sh "${shellBin} notifications clear";
          "Mod+Escape".action = spawn (lib.getExe pkgs.wlogout);
          "Ctrl+Shift+Escape".action = spawn (lib.getExe pkgs.resources);
          "Mod+G".action =
            spawn-sh "pkill ${audioControlName} || ${termBin} --class=audioControl --command=${audioControlBin} -m 100";

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
        };
      };

      home.packages = with pkgs; [
        brightnessctl
        grim
        slurp
        wl-clipboard
        wl-screenrec
        swaylock
        swayidle
        mission-center
        xwayland-satellite
      ];

      home.sessionVariables = {
        NIXOS_OZONE_WL = "1";
        MOZ_ENABLE_WAYLAND = "1";
        QT_QPA_PLATFORM = "wayland";
        QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
        SDL_VIDEODRIVER = "wayland";
        _JAVA_AWT_WM_NONREPARENTING = "1";
      };
    };
}
