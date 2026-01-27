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
        bww

        # Misc desktop apps
        vdhcoapp
        wowup-cf
        yubikey-manager
        qbittorrent
        sqlit-tui
        bitwarden-desktop
        bitwarden-cli
        gnome-calculator
        gnome-calendar
        gnome-clocks
        mpris-timer
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
