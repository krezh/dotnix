{ pkgs, ... }:

{
  home = {
    pointerCursor = {
      gtk.enable = true;
      x11.enable = true;
      # package = pkgs.catppuccin-cursors;
      # name = "mochaLight";
      # size = 28;
    };
  };

  catppuccin.pointerCursor.enable = true;
  catppuccin.pointerCursor.flavor = "mocha";
  catppuccin.pointerCursor.accent = "light";

  dconf.settings = {
    "org/gnome/desktop/interface" = {
      color-scheme = "prefer-dark";
    };
  };

  gtk = {
    enable = true;
    catppuccin.icon.enable = true;
    theme = {
      name = "Arc-Dark";
      package = pkgs.arc-theme;
    };
    # iconTheme = {
    # name = "Papirus-Dark";
    # package = pkgs.papirus-icon-theme;
    # };
  };
}
