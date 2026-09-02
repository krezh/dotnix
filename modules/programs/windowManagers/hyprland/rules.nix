{
  flake.modules.homeManager.hyprland =
    { config, lib, ... }:
    let
      inherit (import ./_helpers.nix { inherit lib; }) expandRules mkRules;

      # Shared by both dialog tags below.
      dialogLook = {
        float = true;
        size = "(monitor_w*0.5) (monitor_h*0.5)";
        center = true;
      };
    in
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

          layer_rule = expandRules [
            {
              match.namespace = "^(rofi)$";
              blur = true;
            }
            {
              match.namespace = "^(launcher)$";
              animation = "popin 80%";
              blur = true;
            }
            {
              match.namespace = "^(walker)$";
              animation = "popin 60%";
              blur = true;
            }
            {
              match.namespace = [
                "^(hyprpicker)$"
                "^(logout_dialog)$"
                "^(chomp-selection)$"
                "^(wayfreeze)$"
              ];
              animation = "fade";
            }
            {
              match.namespace = "^(noctalia:.*)$";
              no_anim = true;
            }
          ];

          window_rule = mkRules {
            # Each tag: how a window earns it, and what that gets it.
            # anyOf — any one field is enough. allOf — all fields together.
            tags = {
              games = {
                anyOf = {
                  class = [
                    "^(gamescope)$"
                    "^(steam_proton)$"
                    "^(steam_app_default)$"
                    "^(steam_app_[0-9]+)$"
                  ];
                  xdg_tag = "^(proton-game)$";
                  content = 3;
                };
                apply = {
                  workspace = "3";
                  idle_inhibit = "always";
                  opacity = "1.0 override";
                  no_blur = true;
                  render_unfocused = true;
                };
              };

              browsers = {
                anyOf.class = [
                  "^(zen.*)$"
                  "^(firefox)$"
                  "^(chromium)$"
                  "^(chrome)$"
                  "^(vivaldi-stable)$"
                  "^(helium)$"
                  "^(brave-browser)$"
                ];
                apply.opacity = "1.0 override";
              };

              media = {
                anyOf = {
                  class = [
                    "^(mpv)$"
                    "^(plex)$"
                    "^(org.jellyfin.JellyfinDesktop)$"
                  ];
                  content = [
                    1
                    2
                  ];
                };
                apply = {
                  opacity = "1.0 override";
                  no_blur = true;
                };
              };

              chat = {
                anyOf.class = [
                  "^(vesktop)$"
                  "^(legcord)$"
                  "^(discord)$"
                ];
                apply.workspace = "4 silent";
              };

              # The shared GTK file picker — every window of this class is a
              # dialog, whatever it is called.
              portalDialog = {
                anyOf.class = "^(xdg-desktop-portal-gtk)$";
                apply = dialogLook;
              };

              # Apps that draw their own dialogs instead of using the picker,
              # so they can only be spotted by title. Anchored, or a title that
              # merely contains "Library" or "Save As" would float too.
              appDialog = {
                anyOf.title = [
                  "^(Select|Open)( a)? (File|Folder)(s)?$"
                  "^File (Operation|Upload)( Progress)?$"
                  "^.* Properties$"
                  "^Export Image as PNG$"
                  "^GIMP Crash Debug$"
                  "^Save As$"
                  "^Library$"
                  "^Select the game's \\.exe$"
                ];
                apply = dialogLook;
              };
            };

            # Standalone rules, applied in order. Fields in `match` are AND.
            rules = [
              # No border on a window that is alone on its workspace.
              {
                match = {
                  float = false;
                  workspace = [
                    "w[tv1]s[false]"
                    "f[1]s[false]"
                  ];
                };
                border_size = 0;
              }
              # Fullscreen windows stay opaque and block idle.
              {
                match.fullscreen = true;
                opacity = "1.0 override";
                idle_inhibit = "fullscreen";
              }
              # Xwayland popups: no dim, no shadow.
              {
                match = {
                  xwayland = true;
                  title = "win[0-9]+";
                };
                no_dim = true;
                no_shadow = true;
                rounding = config.var.rounding;
              }
              # Discord's popped-out window.
              {
                match.initial_title = "^(Discord Popout)$";
                opacity = "1.0 override";
              }
              # Password and launcher prompts keep focus.
              {
                match.class = [
                  "(pinentry-)(.*)"
                  "(Rofi)"
                ];
                stay_focused = true;
              }
              # Archive manager floats.
              {
                match.class = [
                  "org.gnome.FileRoller"
                  "file-roller"
                ];
                float = true;
              }
              # Image viewer floats.
              {
                match.class = "^(org.libvips.vipsdisp)$";
                float = true;
              }
              # Anything floating gets centred.
              {
                match.float = true;
                center = true;
              }
              # Floating terminal.
              {
                match.class = [
                  "floatTerm"
                  "com.floatterm.floatterm"
                ];
                float = true;
                size = "(monitor_w*0.5) (monitor_h*0.5)";
              }
              # System monitor floats, pinned and centred.
              {
                match.class = "(net.nokyan.Resources)";
                float = true;
                pin = true;
                center = true;
                size = "(monitor_w*0.5) (monitor_h*0.5)";
              }
            ];
          };
        };
      };
    };
}
