{
  flake.modules.homeManager.niri =
    {
      config,
      lib,
      osConfig,
      ...
    }:
    {
      wayland.windowManager.niri.settings._children =
        # Host-specific workspace output assignments (matching Hyprland workspace_rule)
        (lib.optionals (osConfig.networking.hostName == "thor") [
          {
            workspace = {
              _args = [ "1" ];
              open-on-output = "DP-1";
            };
          }
          {
            workspace = {
              _args = [ "2" ];
              open-on-output = "DP-1";
            };
          }
          {
            workspace = {
              _args = [ "3" ];
              open-on-output = "DP-1";
            };
          }
          {
            workspace = {
              _args = [ "4" ];
              open-on-output = "DP-2";
            };
          }
          {
            workspace = {
              _args = [ "5" ];
              open-on-output = "DP-2";
            };
          }
          {
            workspace = {
              _args = [ "6" ];
              open-on-output = "DP-2";
            };
          }
        ])
        ++ [
          # Global window corner rounding
          {
            window-rule = {
              geometry-corner-radius = config.var.rounding;
              clip-to-geometry = true;
            };
          }

          # Inactive window opacity
          {
            window-rule = {
              match._props = {
                is-active = false;
              };
              opacity = config.var.opacity;
            };
          }

          # Games: open on workspace 3, full opacity, VRR enabled
          {
            window-rule = {
              match._props = {
                app-id = "^(gamescope|steam_proton|steam_app_default|steam_app_[0-9]+)$";
              };
              open-on-workspace = "3";
              opacity = 1.0;
              variable-refresh-rate = true;
            };
          }

          # Browsers: full opacity override
          {
            window-rule = {
              match._props = {
                app-id = "^(zen.*|firefox|chromium|chrome|vivaldi-stable|helium|brave-browser)$";
              };
              opacity = 1.0;
            };
          }

          # Media players: full opacity override
          {
            window-rule = {
              match._props = {
                app-id = "^(mpv|plex|org\\.jellyfin\\.JellyfinDesktop)$";
              };
              opacity = 1.0;
            };
          }

          # Chat: open on workspace 4 silently (without stealing focus)
          {
            window-rule = {
              match._props = {
                app-id = "^(vesktop|legcord|discord)$";
              };
              open-on-workspace = "4";
              open-focused = false;
            };
          }

          # Shared GTK file picker dialog
          {
            window-rule = {
              match._props = {
                app-id = "^(xdg-desktop-portal-gtk)$";
              };
              open-floating = true;
              default-column-width.proportion = 0.5;
            };
          }

          # Application dialogs matched by title
          {
            window-rule = {
              match._props = {
                title = "^((Select|Open)( a)? (File|Folder)(s)?|File (Operation|Upload)( Progress)?|.* Properties|Export Image as PNG|GIMP Crash Debug|Save As|Library|Select the game's \\.exe)$";
              };
              open-floating = true;
              default-column-width.proportion = 0.5;
            };
          }

          # Password and launcher prompts keep focus
          {
            window-rule = {
              match._props = {
                app-id = "^(pinentry.*|Rofi)$";
              };
              open-floating = true;
              open-focused = true;
            };
          }

          # Floating terminal
          {
            window-rule = {
              match._props = {
                app-id = "^(floatTerm|com\\.floatterm\\.floatterm)$";
              };
              open-floating = true;
              default-column-width.proportion = 0.5;
            };
          }

          # System monitor (resources)
          {
            window-rule = {
              match._props = {
                app-id = "^(net\\.nokyan\\.Resources)$";
              };
              open-floating = true;
              default-column-width.proportion = 0.5;
            };
          }

          # Archive manager and image viewer
          {
            window-rule = {
              match._props = {
                app-id = "^(org\\.gnome\\.FileRoller|file-roller|org\\.libvips\\.vipsdisp)$";
              };
              open-floating = true;
            };
          }

          # Firefox Picture-in-Picture
          {
            window-rule = {
              match._props = {
                app-id = "firefox$";
                title = "^Picture-in-Picture$";
              };
              open-floating = true;
            };
          }

          # Discord Popout
          {
            window-rule = {
              match._props = {
                title = "^(Discord Popout)$";
              };
              opacity = 1.0;
            };
          }

          # Layer rules: Launchers blur and rounded corners
          {
            layer-rule = {
              match._props = {
                namespace = "^(rofi|launcher|walker)$";
              };
              background-effect.blur = true;
              geometry-corner-radius = config.var.rounding;
            };
          }

          # Layer rules: Screen capture and overlay tools without blur
          {
            layer-rule = {
              match._props = {
                namespace = "^(hyprpicker|logout_dialog|chomp-selection|wayfreeze)$";
              };
              background-effect.blur = false;
            };
          }
        ];
    };
}
