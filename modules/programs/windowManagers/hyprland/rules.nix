{
  flake.modules.homeManager.hyprland =
    { config, lib, ... }:
    let
      # mkRules turns one config attrset into Hyprland's flat window-rule list:
      #   tags.<name>  — a window earns the "+<name>" tag if it matches any field
      #                  listed here (each field/value becomes its own rule = OR).
      #                  A list value is OR within that field; `all = [ { … } ]`
      #                  requires every field of a set together (AND).
      #   apply.<name> — properties applied to any window carrying that tag.
      #   rules        — ordered list of `{ label = { match = …; …props }; }` for
      #                  non-tag rules; order is preserved (Hyprland applies them
      #                  top-to-bottom). List fields in any `match` expand to
      #                  one rule per value.
      inherit (import ./_helpers.nix { inherit lib; }) expandRules mkRules;
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
            # Tag definitions — each field/value is an independent way to earn the tag.
            tags = {
              games = {
                class = [
                  "^(gamescope)$"
                  "^(steam_proton)$"
                  "^(steam_app_default)$"
                  "^(steam_app_[0-9]+)$"
                ];
                xdg_tag = "^(proton-game)$";
                content = 3;
              };
              browsers.class = [
                "^(zen.*)$"
                "^(firefox)$"
                "^(chromium)$"
                "^(chrome)$"
                "^(vivaldi-stable)$"
                "^(helium)$"
              ];
              media = {
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
              chat.class = [
                "^(vesktop)$"
                "^(legcord)$"
                "^(discord)$"
              ];
              dialog = {
                class = "xdg-desktop-portal-gtk";
                title = [
                  "(Select|Open)( a)? (File|Folder)(s)?"
                  "File (Operation|Upload)( Progress)?"
                  ".* Properties"
                  "Export Image as PNG"
                  "GIMP Crash Debug"
                  "Save As"
                  "Library"
                ];
                all = [
                  {
                    title = "Install";
                    class = "steam";
                    modal = true;
                  }
                ];
              };
            };

            # Properties applied to windows carrying a tag.
            apply = {
              chat.workspace = "4 silent";
              browsers.opacity = "1.0 override";
              media = {
                opacity = "1.0 override";
                no_blur = true;
              };
              games = {
                workspace = "3";
                idle_inhibit = "always";
                opacity = "1.0 override";
                no_blur = true;
                render_unfocused = true;
                immediate = true;
              };
              dialog = {
                float = true;
                size = "(monitor_w*0.5) (monitor_h*0.5)";
                center = true;
              };
            };

            # Standalone rules (no tag) — ordered; each labeled.
            rules = [
              {
                smartBorders = {
                  match = {
                    float = false;
                    workspace = [
                      "w[tv1]s[false]"
                      "f[1]s[false]"
                    ];
                  };
                  border_size = 0;
                };
              }
              {
                fullscreen = {
                  match.fullscreen = true;
                  opacity = "1.0 override";
                  idle_inhibit = "fullscreen";
                };
              }
              {
                xwaylandPopups = {
                  match = {
                    xwayland = true;
                    title = "win[0-9]+";
                  };
                  no_dim = true;
                  no_shadow = true;
                  rounding = config.var.rounding;
                };
              }
              {
                discordPopout = {
                  match.initial_title = "^(Discord Popout)$";
                  opacity = "1.0 override";
                };
              }
              {
                stayFocused = {
                  match.class = [
                    "(pinentry-)(.*)"
                    "(Rofi)"
                  ];
                  stay_focused = true;
                };
              }
              {
                fileManagers = {
                  match.class = [
                    "org.gnome.FileRoller"
                    "file-roller"
                  ];
                  float = true;
                };
              }
              {
                vips = {
                  match.class = "^(org.libvips.vipsdisp)$";
                  float = true;
                };
              }
              {
                floatCentering = {
                  match.float = true;
                  center = true;
                };
              }
              {
                floatTerminal = {
                  match.class = [
                    "floatTerm"
                    "com.floatterm.floatterm"
                  ];
                  float = true;
                  size = "(monitor_w*0.5) (monitor_h*0.5)";
                };
              }
              {
                resources = {
                  match.class = "(net.nokyan.Resources)";
                  float = true;
                  pin = true;
                  center = true;
                  size = "(monitor_w*0.5) (monitor_h*0.5)";
                };
              }
            ];
          };
        };
      };
    };
}
