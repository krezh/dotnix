{
  flake.modules.homeManager.hyprland =
    { config, ... }:
    {
      wayland.windowManager.hyprland = {
        settings = {
          workspace_rule = [
            {
              workspace = "1";
              monitor = "DP-1";
            }
            {
              workspace = "2";
              monitor = "DP-1";
            }
            {
              workspace = "3";
              monitor = "DP-1";
            }
            {
              workspace = "4";
              monitor = "DP-2";
            }
            {
              workspace = "5";
              monitor = "DP-2";
            }
            {
              workspace = "6";
              monitor = "DP-2";
            }
          ];

          layer_rule = [
            {
              match = {
                namespace = "^(rofi)$";
              };
              blur = true;
            }
            {
              match = {
                namespace = "^(launcher)$";
              };
              animation = "popin 80%";
            }
            {
              match = {
                namespace = "^(launcher)$";
              };
              blur = true;
            }
            {
              match = {
                namespace = "^(walker)$";
              };
              animation = "popin 60%";
            }
            {
              match = {
                namespace = "^(walker)$";
              };
              blur = true;
            }
            {
              match = {
                namespace = "^(hyprpicker)$";
              };
              animation = "fade";
            }
            {
              match = {
                namespace = "^(logout_dialog)$";
              };
              animation = "fade";
            }
            {
              match = {
                namespace = "^(chomp-selection)$";
              };
              animation = "fade";
            }
            {
              match = {
                namespace = "^(wayfreeze)$";
              };
              animation = "fade";
            }
            {
              match = {
                namespace = "^(noctalia:.*)$";
              };
              no_anim = true;
            }
          ];

          window_rule = [
            # Tag definitions
            {
              match = {
                class = "^(gamescope)$";
              };
              tag = "+games";
            }
            {
              match = {
                class = "^(steam_proton)$";
              };
              tag = "+games";
            }
            {
              match = {
                class = "^(steam_app_default)$";
              };
              tag = "+games";
            }
            {
              match = {
                class = "^(steam_app_[0-9]+)$";
              };
              tag = "+games";
            }
            {
              match = {
                xdg_tag = "^(proton-game)$";
              };
              tag = "+games";
            }
            {
              match = {
                content = 3;
              };
              tag = "+games";
            }
            {
              match = {
                class = "^(zen.*)$";
              };
              tag = "+browsers";
            }
            {
              match = {
                class = "^(firefox)$";
              };
              tag = "+browsers";
            }
            {
              match = {
                class = "^(chromium)$";
              };
              tag = "+browsers";
            }
            {
              match = {
                class = "^(chrome)$";
              };
              tag = "+browsers";
            }
            {
              match = {
                class = "^(vivaldi-stable)$";
              };
              tag = "+browsers";
            }
            {
              match = {
                class = "^(helium)$";
              };
              tag = "+browsers";
            }
            {
              match = {
                class = "^(mpv)$";
              };
              tag = "+media";
            }
            {
              match = {
                class = "^(plex)$";
              };
              tag = "+media";
            }
            {
              match = {
                class = "^(org.jellyfin.JellyfinDesktop)$";
              };
              tag = "+media";
            }
            {
              match = {
                content = 2;
              };
              tag = "+media";
            }
            {
              match = {
                content = 1;
              };
              tag = "+media";
            }
            {
              match = {
                class = "^(vesktop)$";
              };
              tag = "+chat";
            }
            {
              match = {
                class = "^(legcord)$";
              };
              tag = "+chat";
            }
            {
              match = {
                class = "^(discord)$";
              };
              tag = "+chat";
            }
            {
              match = {
                title = "(Select|Open)( a)? (File|Folder)(s)?";
              };
              tag = "+dialog";
            }
            {
              match = {
                title = "File (Operation|Upload)( Progress)?";
              };
              tag = "+dialog";
            }
            {
              match = {
                class = "xdg-desktop-portal-gtk";
              };
              tag = "+dialog";
            }
            {
              match = {
                title = ".* Properties";
              };
              tag = "+dialog";
            }
            {
              match = {
                title = "Export Image as PNG";
              };
              tag = "+dialog";
            }
            {
              match = {
                title = "GIMP Crash Debug";
              };
              tag = "+dialog";
            }
            {
              match = {
                title = "Save As";
              };
              tag = "+dialog";
            }
            {
              match = {
                title = "Library";
              };
              tag = "+dialog";
            }
            {
              match = {
                title = "Install";
                class = "steam";
              };
              tag = "+dialog";
            }
            {
              match = {
                modal = true;
              };
              tag = "+dialog";
            }

            # Tag rules
            {
              match = {
                tag = "chat";
              };
              workspace = "4 silent";
            }
            {
              match = {
                tag = "browsers";
              };
              opacity = "1.0 override";
            }
            {
              match = {
                tag = "media";
              };
              opacity = "1.0 override";
            }
            {
              match = {
                tag = "media";
              };
              no_blur = true;
            }
            {
              match = {
                tag = "games";
              };
              workspace = "3";
            }
            {
              match = {
                tag = "games";
              };
              idle_inhibit = "always";
            }
            {
              match = {
                tag = "games";
              };
              opacity = "1.0 override";
            }
            {
              match = {
                tag = "games";
              };
              no_blur = true;
            }
            {
              match = {
                tag = "games";
              };
              render_unfocused = true;
            }
            {
              match = {
                tag = "games";
              };
              immediate = true;
            }
            {
              match = {
                tag = "dialog";
              };
              float = true;
            }
            {
              match = {
                tag = "dialog";
              };
              size = "(monitor_w*0.5) (monitor_h*0.5)";
            }
            {
              match = {
                tag = "dialog";
              };
              center = true;
            }

            # Smart borders
            {
              match = {
                float = false;
                workspace = "w[tv1]s[false]";
              };
              border_size = 0;
            }
            {
              match = {
                float = false;
                workspace = "f[1]s[false]";
              };
              border_size = 0;
            }

            # Fullscreen
            {
              match = {
                fullscreen = true;
              };
              opacity = "1.0 override";
            }
            {
              match = {
                fullscreen = true;
              };
              idle_inhibit = "fullscreen";
            }

            # XWayland popups
            {
              match = {
                xwayland = true;
                title = "win[0-9]+";
              };
              no_dim = true;
            }
            {
              match = {
                xwayland = true;
                title = "win[0-9]+";
              };
              no_shadow = true;
            }
            {
              match = {
                xwayland = true;
                title = "win[0-9]+";
              };
              rounding = config.var.rounding;
            }

            # Opacity overrides
            {
              match = {
                initial_title = "^(Discord Popout)$";
              };
              opacity = "1.0 override";
            }

            # Stay focused
            {
              match = {
                class = "(pinentry-)(.*)";
              };
              stay_focused = true;
            }
            {
              match = {
                class = "(Rofi)";
              };
              stay_focused = true;
            }

            # File managers
            {
              match = {
                class = "org.gnome.FileRoller";
              };
              float = true;
            }
            {
              match = {
                class = "file-roller";
              };
              float = true;
            }

            # Vips
            {
              match = {
                class = "^(org.libvips.vipsdisp)$";
              };
              float = true;
            }

            # Float centering
            {
              match = {
                float = true;
              };
              center = true;
            }

            # Float terminal
            {
              match = {
                class = "floatTerm";
              };
              float = true;
            }
            {
              match = {
                class = "floatTerm";
              };
              size = "(monitor_w*0.5) (monitor_h*0.5)";
            }
            {
              match = {
                class = "com.floatterm.floatterm";
              };
              float = true;
            }
            {
              match = {
                class = "com.floatterm.floatterm";
              };
              size = "(monitor_w*0.5) (monitor_h*0.5)";
            }

            # Resources
            {
              match = {
                class = "(net.nokyan.Resources)";
              };
              float = true;
            }
            {
              match = {
                class = "(net.nokyan.Resources)";
              };
              pin = true;
            }
            {
              match = {
                class = "(net.nokyan.Resources)";
              };
              center = true;
            }
            {
              match = {
                class = "(net.nokyan.Resources)";
              };
              size = "(monitor_w*0.5) (monitor_h*0.5)";
            }
          ];
        };
      };
    };
}
