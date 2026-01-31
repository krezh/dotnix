{
  flake.modules.homeManager.desktop-utils =
    { pkgs, ... }:
    {
      home.packages = with pkgs; [
        # Screen/clipboard utilities
        brightnessctl
        grim
        wl-clipboard
        wl-screenrec
        recshot
        gulp

        mission-center

        # Misc desktop apps
        wowup-cf
        yubikey-manager
        qbittorrent
        sqlit-tui
        mpris-timer
        gnome-calculator
        gnome-calendar
        gnome-clocks
        gnome-bluetooth
        gnome-maps
        gnome-online-accounts-gtk
        geary
        file-roller
        proton-pass
        evince
      ];
    };
}
