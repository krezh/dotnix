_: {
  flake.modules.homeManager.niri =
    { config, ... }:
    {
      programs.niri.settings.window-rules = [
        {
          geometry-corner-radius = {
            bottom-left = 15.0;
            bottom-right = 15.0;
            top-left = 15.0;
            top-right = 15.0;
          };
          clip-to-geometry = true;
          draw-border-with-background = false;
          inherit (config.var) opacity;
          shadow.enable = true;
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
          "default-column-width".proportion = 0.6;
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
          matches = [ { "app-id" = "^plex$"; } ];
          opacity = 1.0;
        }
      ];
    };
}
