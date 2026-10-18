{
  flake.modules.homeManager.umbriel =
    {
      config,
      lib,
      osConfig,
      ...
    }:
    let
      isThor = osConfig.networking.hostName == "thor";
      floatingHalfWidth = {
        default_floating = true;
        default_floating_size.width = 0.5;
      };
    in
    {
      programs.umbriel.settings = {
        workspace = lib.optionals isThor [
          {
            name = "1";
            output = "DP-1";
          }
          {
            name = "2";
            output = "DP-1";
          }
          {
            name = "3";
            output = "DP-1";
          }
          {
            name = "4";
            output = "DP-2";
          }
          {
            name = "5";
            output = "DP-2";
          }
          {
            name = "6";
            output = "DP-2";
          }
        ];

        window_rule = [
          {
            match.is_focused = false;
            opacity = config.var.opacity;
          }
          (
            {
              match.app_id = "^(gamescope|steam_proton|steam_app_default|steam_app_[0-9]+)$";
              default_workspace = if isThor then "3" else 3;
              opacity = 1.0;
              vrr = "always";
            }
            // lib.optionalAttrs isThor { default_output = "DP-1"; }
          )
          {
            match.app_id = "^(zen.*|firefox|chromium|chrome|vivaldi-stable|helium|brave-browser)$";
            opacity = 1.0;
          }
          {
            match.app_id = "^(mpv|plex|org\\.jellyfin\\.JellyfinDesktop)$";
            opacity = 1.0;
          }
          (
            {
              match.app_id = "^(vesktop|legcord|discord)$";
              default_workspace = if isThor then "4" else 4;
              default_focused = false;
            }
            // lib.optionalAttrs isThor { default_output = "DP-2"; }
          )
          (floatingHalfWidth // { match.app_id = "^xdg-desktop-portal-gtk$"; })
          (
            floatingHalfWidth
            // {
              match.title = "^((Select|Open)( a)? (File|Folder)(s)?|File (Operation|Upload)( Progress)?|.* Properties|Export Image as PNG|GIMP Crash Debug|Save As|Library|Select the game's \\.exe)$";
            }
          )
          {
            match.app_id = "^(pinentry.*|Rofi)$";
            default_floating = true;
            default_focused = true;
          }
          (floatingHalfWidth // { match.app_id = "^(floatTerm|com\\.floatterm\\.floatterm)$"; })
          (floatingHalfWidth // { match.app_id = "^net\\.nokyan\\.Resources$"; })
          {
            match.app_id = "^(org\\.gnome\\.FileRoller|file-roller|org\\.libvips\\.vipsdisp)$";
            default_floating = true;
          }
          {
            match = {
              app_id = "^firefox$";
              title = "^Picture-in-Picture$";
            };
            default_floating = true;
          }
          {
            match.title = "^Discord Popout$";
            opacity = 1.0;
          }
        ];

        layer_rule = [
          {
            match.namespace = "^(rofi|launcher|walker)$";
            blur = true;
          }
          {
            match.namespace = "^(hyprpicker|logout_dialog|chomp-selection|wayfreeze)$";
            blur = false;
          }
        ];
      };
    };
}
