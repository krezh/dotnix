{
  flake.modules.homeManager.hyprland =
    { config, ... }:
    {
      wayland.windowManager.hyprland = {
        settings = {
          workspace = [
            "1,monitor:DP-1"
            "2,monitor:DP-1"
            "3,monitor:DP-1"
            "4,monitor:DP-2"
            "5,monitor:DP-2"
            "6,monitor:DP-2"
          ];

          layerrule = [
            "match:namespace ^(rofi)$, blur on"
            "match:namespace ^(launcher)$, animation popin 80%"
            "match:namespace ^(launcher)$, blur on"
            "match:namespace ^(walker)$, animation popin 60%"
            "match:namespace ^(walker)$, blur on"
            "match:namespace ^(hyprpicker)$, animation fade"
            "match:namespace ^(logout_dialog)$, animation fade"
            "match:namespace ^(chomp-selection)$, animation fade"
            "match:namespace ^(wayfreeze)$, animation fade"
            "match:namespace ^(noctalia:.*)$, no_anim on"
          ];

          windowrule = [
            # Tags Definitions
            "tag +games, match:class ^(gamescope)$"
            "tag +games, match:class ^(steam_proton)$"
            "tag +games, match:class ^(steam_app_default)$"
            "tag +games, match:class ^(steam_app_[0-9]+)$"
            "tag +games, match:xdg_tag ^(proton-game)$"
            "tag +games, match:content 3" # (none = 0, photo = 1, video = 2, game = 3)
            "tag +browsers, match:class ^(zen.*)$"
            "tag +browsers, match:class ^(firefox)$"
            "tag +browsers, match:class ^(chromium)$"
            "tag +browsers, match:class ^(chrome)$"
            "tag +browsers, match:class ^(vivaldi-stable)$"
            "tag +browsers, match:class ^(helium)$"
            "tag +media, match:class ^(mpv)$"
            "tag +media, match:class ^(plex)$"
            "tag +media, match:class ^(org.jellyfin.JellyfinDesktop)$"
            "tag +media, match:content 2" # (none = 0, photo = 1, video = 2, game = 3)
            "tag +media, match:content 1" # (none = 0, photo = 1, video = 2, game = 3)
            "tag +chat, match:class ^(vesktop)$"
            "tag +chat, match:class ^(legcord)$"
            "tag +chat, match:class ^(discord)$"
            "tag +dialog, match:title (Select|Open)( a)? (File|Folder)(s)?"
            "tag +dialog, match:title File (Operation|Upload)( Progress)?"
            "tag +dialog, match:class xdg-desktop-portal-gtk"
            "tag +dialog, match:title .* Properties"
            "tag +dialog, match:title Export Image as PNG"
            "tag +dialog, match:title GIMP Crash Debug"
            "tag +dialog, match:title Save As"
            "tag +dialog, match:title Library"
            "tag +dialog, match:title Install, match:class steam"
            "tag +dialog, match:modal true"

            # Tag Rules
            # Chat
            "match:tag chat, workspace 4 silent"
            # Browsers
            "match:tag browsers, opacity 1.0 override"
            # Media
            "match:tag media, opacity 1.0 override"
            "match:tag media, no_blur on"
            # Games
            "match:tag games, workspace 3"
            "match:tag games, idle_inhibit always"
            "match:tag games, opacity 1.0 override"
            "match:tag games, no_blur on"
            "match:tag games, render_unfocused on"
            "match:tag games, immediate true"
            # Dialog
            "match:tag dialog, float on"
            "match:tag dialog, size (monitor_w*0.5) (monitor_h*0.5)"
            "match:tag dialog, center on"

            # Smart borders
            "match:float false, match:workspace w[tv1]s[false], border_size 0"
            "match:float false, match:workspace f[1]s[false], border_size 0"

            # Fullscreen
            "match:fullscreen true, opacity 1.0 override"
            "match:fullscreen true, idle_inhibit fullscreen"

            # XWayland popups
            "match:xwayland true, match:title win[0-9]+, no_dim on"
            "match:xwayland true, match:title win[0-9]+, no_shadow on"
            "match:xwayland true, match:title win[0-9]+, rounding ${toString config.var.rounding}"

            # Opacity overrides
            "match:initial_title ^(Discord Popout)$, opacity 1.0 override"

            # pinentry
            "match:class (pinentry-)(.*), stay_focused on"

            # Rofi
            "match:class (Rofi), stay_focused on"

            # File managers
            "match:class org.gnome.FileRoller, float on"
            "match:class file-roller, float on"

            # Vips
            "match:class ^(org.libvips.vipsdisp)$, float on"

            "match:float true, center on"

            # Float Terminal
            "match:class floatTerm, float on"
            "match:class floatTerm, size (monitor_w*0.5) (monitor_h*0.5)"
            "match:class com.floatterm.floatterm, float on"
            "match:class com.floatterm.floatterm, size (monitor_w*0.5) (monitor_h*0.5)"

            # Resources
            "match:class (net.nokyan.Resources), float on"
            "match:class (net.nokyan.Resources), pin on"
            "match:class (net.nokyan.Resources), center on"
            "match:class (net.nokyan.Resources), size (monitor_w*0.5) (monitor_h*0.5)"
          ];
        };
      };
    };
}
