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
        gnome-calculator
        gnome-calendar
        gnome-clocks
        gnome-bluetooth
        gnome-online-accounts-gtk
        gnome-disk-utility
        geary
        file-roller
        proton-pass
        proton-pass-cli
        evince
      ];
    };
}
