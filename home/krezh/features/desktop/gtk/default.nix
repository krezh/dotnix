{ pkgs, ... }:

{
  # home = {
  #   pointerCursor = {
  #     gtk.enable = true;
  #     x11.enable = true;
  #     package = pkgs.catppuccin-cursors;
  #     name = "mochaBlue";
  #     #size = 28;
  #   };
  # };

  gtk = {
    enable = true;
    # theme = {
    #   name = "Arc-Dark";
    #   package = pkgs.arc-theme;
    # };
    iconTheme = {
      name = "Papirus-Dark";
      package = pkgs.papirus-icon-theme;
    };
  };
}
