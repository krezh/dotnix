_: {
  flake.modules.homeManager.gnome =
    {
      pkgs,
      lib,
      config,
      ...
    }:
    {
      home.packages = with pkgs; [
        gnomeExtensions.appindicator
        gnomeExtensions.blur-my-shell
        gnomeExtensions.dash-to-panel
        gnomeExtensions.just-perfection
        gnomeExtensions.caffeine
        gnomeExtensions.vitals
        gnomeExtensions.rounded-window-corners-reborn
      ];

      # Random wallpaper service
      systemd.user.services.gnome-random-wallpaper = {
        Unit = {
          Description = "Change GNOME wallpaper randomly";
          PartOf = [ "gnome-session.target" ];
        };
        Service = {
          Type = "oneshot";
          ExecStart = "${pkgs.writeShellScript "change-wallpaper" ''
            WALLPAPER_DIR="${config.home.file.wallpapers.source}"
            if [ -d "$WALLPAPER_DIR" ]; then
              WALLPAPER=$(find "$WALLPAPER_DIR" -type f \( -name "*.jpg" -o -name "*.png" -o -name "*.jpeg" \) | shuf -n 1)
              if [ -n "$WALLPAPER" ]; then
                gsettings set org.gnome.desktop.background picture-uri "file://$WALLPAPER"
                gsettings set org.gnome.desktop.background picture-uri-dark "file://$WALLPAPER"
              fi
            fi
          ''}";
        };
      };

      systemd.user.timers.gnome-random-wallpaper = {
        Unit = {
          Description = "Change GNOME wallpaper timer";
          PartOf = [ "gnome-session.target" ];
        };
        Timer = {
          OnBootSec = "1min";
          OnUnitActiveSec = "15min";
        };
        Install = {
          WantedBy = [ "gnome-session.target" ];
        };
      };

      dconf.enable = true;
      dconf.settings = {
        # Extensions
        "org/gnome/shell" = {
          disable-user-extensions = false;
          enabled-extensions = [
            pkgs.gnomeExtensions.appindicator.extensionUuid
            pkgs.gnomeExtensions.blur-my-shell.extensionUuid
            pkgs.gnomeExtensions.dash-to-panel.extensionUuid
            pkgs.gnomeExtensions.just-perfection.extensionUuid
            pkgs.gnomeExtensions.caffeine.extensionUuid
            pkgs.gnomeExtensions.vitals.extensionUuid
            pkgs.gnomeExtensions.rounded-window-corners-reborn.extensionUuid
          ];
        };

        # Blur My Shell Configuration
        "org/gnome/shell/extensions/blur-my-shell" = {
          brightness = 0.75;
          noise-amount = 0.0;
          sigma = 30; # Blur intensity
        };

        "org/gnome/shell/extensions/blur-my-shell/panel" = {
          blur = true;
          override-background = true;
          style-panel = 0; # Transparent
        };

        "org/gnome/shell/extensions/blur-my-shell/overview" = {
          blur = true;
          style-components = 0; # Transparent
        };

        "org/gnome/shell/extensions/blur-my-shell/appfolder" = {
          blur = true;
          style-dialogs = 0; # Transparent
        };

        "org/gnome/shell/extensions/blur-my-shell/window-list" = {
          blur = true;
        };

        # Dash to Panel Configuration
        "org/gnome/shell/extensions/dash-to-panel" = {
          panel-position = "BOTTOM";
          panel-size = 48;
          appicon-margin = 4;
          appicon-padding = 4;
          appicon-style = "NORMAL";
          show-favorites = true;
          show-running-apps = true;
          isolate-workspaces = false;
          isolate-monitors = true;
          show-show-apps-button = true;
          show-activities-button = false;
          animate-app-switch = true;
          animate-appicon-hover = true;
          dot-color-dominant = true;
          dot-color-override = false;
          dot-color-unfocused-different = false;
          dot-position = "BOTTOM";
          dot-style-focused = "SEGMENTED";
          dot-style-unfocused = "DOTS";
          focus-highlight = false;
          global-border-radius = 15;
          highlight-appicon-hover = false;
          hotkeys-overlay-combo = "NEVER";
          window-preview-title-position = "TOP";
          trans-use-border = false;
          trans-use-custom-bg = false;
          trans-use-custom-gradient = false;
          trans-use-custom-opacity = true;
          trans-use-dynamic-opacity = false;
        };

        # Just Perfection Configuration
        "org/gnome/shell/extensions/just-perfection" = {
          animation = 1;
          panel = true;
          panel-in-overview = true;
          workspace-popup = false; # Hide workspace switch popup
          startup-status = 0; # Disable startup status
          window-demands-attention-focus = true;
          notification-banner-position = 2;
        };

        # Caffeine Configuration
        "org/gnome/shell/extensions/caffeine" = {
          toggle-shortcut = [ ];
        };

        # Rounded Window Corners Configuration
        "org/gnome/shell/extensions/rounded-window-corners" = {
          border-width = 0;
          corner-radius = 15;
          enabled-custom-presets = [ ];
        };

        # Vitals Configuration
        "org/gnome/shell/extensions/vitals" = {
          hot-sensors = [
            "_processor_usage_"
            "_memory_usage_"
            "__temperature_avg__"
          ];
        };

        # Desktop Interface
        "org/gnome/desktop/interface" = {
          color-scheme = "prefer-dark";
          enable-hot-corners = false;
          clock-show-weekday = true;
          clock-show-seconds = false;
          show-battery-percentage = true;
          enable-animations = true;
        };

        "org/gnome/desktop/calendar" = {
          show-weekdate = true;
        };

        # Window Management
        "org/gnome/desktop/wm/preferences" = {
          button-layout = "appmenu:minimize,maximize,close";
          resize-with-right-button = true;
          mouse-button-modifier = "<Super>";
        };

        # Keybindings - Window Management
        "org/gnome/desktop/wm/keybindings" = {
          close = [ "<Super>q" ];
          toggle-fullscreen = [ "<Super><Shift>f" ];
          toggle-maximized = [ "<Super>f" ];
          minimize = [ "<Super>w" ];

          # Workspace switching
          switch-to-workspace-1 = [ "<Super>1" ];
          switch-to-workspace-2 = [ "<Super>2" ];
          switch-to-workspace-3 = [ "<Super>3" ];
          switch-to-workspace-4 = [ "<Super>4" ];

          # Move windows to workspaces
          move-to-workspace-1 = [ "<Super><Shift>1" ];
          move-to-workspace-2 = [ "<Super><Shift>2" ];
          move-to-workspace-3 = [ "<Super><Shift>3" ];
          move-to-workspace-4 = [ "<Super><Shift>4" ];
        };

        # Custom Keybindings
        "org/gnome/settings-daemon/plugins/media-keys" = {
          custom-keybindings = [
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/"
          ];
        };

        "org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0" = {
          name = "Terminal";
          command = "ghostty";
          binding = "<Super>Return";
        };

        # Input Settings
        "org/gnome/desktop/peripherals/touchpad" = {
          tap-to-click = true;
          two-finger-scrolling-enabled = true;
          natural-scroll = false;
          accel-profile = "flat";
          speed = 0.4;
        };

        "org/gnome/desktop/peripherals/mouse" = {
          accel-profile = "flat";
          natural-scroll = false;
          speed = 0.4;
        };

        # Keyboard Settings
        "org/gnome/desktop/peripherals/keyboard" = {
          numlock-state = true;
        };

        "system/locale" = {
          region = "sv_SE.UTF-8";
        };

        "org/gnome/desktop/input-sources" = {
          sources = [
            (lib.hm.gvariant.mkTuple [
              "xkb"
              "se+nodeadkeys"
            ])
          ];
          xkb-options = [ "numlock:on" ];
        };

        # Display Settings
        "org/gnome/mutter" = {
          edge-tiling = true;
          dynamic-workspaces = false;
          workspaces-only-on-primary = true;
        };

        # Workspaces
        "org/gnome/desktop/wm/preferences" = {
          num-workspaces = 4;
        };

        # Shell Settings
        "org/gnome/shell" = {
          favorite-apps = [
            "org.gnome.Nautilus.desktop"
            "zen-twilight.desktop"
            "com.mitchellh.ghostty.desktop"
          ];
        };

        # Privacy
        "org/gnome/desktop/privacy" = {
          remember-recent-files = false;
          remove-old-temp-files = true;
          remove-old-trash-files = true;
        };

        # Power Settings
        "org/gnome/settings-daemon/plugins/power" = {
          sleep-inactive-ac-type = "nothing";
          power-button-action = "interactive";
        };
      };
    };
}
