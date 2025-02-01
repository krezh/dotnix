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

  catppuccin.cursors.enable = true;
  catppuccin.cursors.flavor = "mocha";
  catppuccin.cursors.accent = "light";
  catppuccin.hyprland.enable = true;
  catppuccin.gtk.icon.enable = true;
  catppuccin.rofi.enable = true;

  dconf.settings = {
    "org/gnome/desktop/interface" = {
      color-scheme = "prefer-dark";
    };
  };

  gtk = {
    enable = true;
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
