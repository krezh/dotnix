{ ... }:
{
  programs.niri.settings.window-rules = [
    # Calculator
    {
      matches = [ { "app-id" = "^org\\.gnome\\.Calculator$"; } ];
      "default-column-width" = {
        fixed = 400;
      };
      "open-floating" = true;
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

    # Nautilus
    {
      matches = [ { "app-id" = "^org\\.gnome\\.Nautilus$"; } ];
      "default-column-width" = {
        proportion = 0.5;
      };
    }

    # Kitty with opacity
    {
      matches = [ { "app-id" = "^kitty$"; } ];
      opacity = 0.85;
    }

    # Zenity
    {
      matches = [ { "app-id" = "^zenity$"; } ];
      "open-floating" = true;
    }

    # Resources (Mission Center)
    {
      matches = [ { "app-id" = "^resources$"; } ];
      "open-floating" = true;
    }

    # File Roller
    {
      matches = [ { "app-id" = "^file-roller$"; } ];
      "open-floating" = true;
    }

    {
      matches = [ { "app-id" = "^org\\.gnome\\.FileRoller$"; } ];
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

    # Bitwarden extension popup
    {
      matches = [ { title = "^.*Bitwarden Password Manager.*$"; } ];
      "open-floating" = true;
      "default-column-width" = {
        proportion = 0.5;
      };
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
}
