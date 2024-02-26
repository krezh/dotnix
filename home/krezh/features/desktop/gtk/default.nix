{ pkgs, ... }:

{
  home = {
    pointerCursor = {
      gtk.enable = true;
      x11.enable = true;
      package = pkgs.catppuccin-cursors;
      name = "mochaBlue";
      size = 28;
    };
    packages = with pkgs; [];
  };

  gtk = {
    enable = true;
    theme = {
      name = "Catppuccin-Mocha-Compact-Maroon-Dark";
      package = pkgs.catppuccin-gtk.override {
        accents = [ "maroon" ];
        size = "compact";
        tweaks = [ "rimless" "black" ];
        variant = "mocha";
      };
    };
  };
}