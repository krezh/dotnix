{ pkgs, ... }:

{
  home = {
    pointerCursor = {
      gtk.enable = true;
      x11.enable = false;
      # package = pkgs.catppuccin-cursors;
      # name = "mochaBlue";
      # size = 28;
    };
  };

  gtk = {
    enable = true;
    theme = {
      name = "Arc-Dark";
      package = pkgs.arc-theme;
    };
  };
}
