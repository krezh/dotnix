{
  flake.modules.homeManager.gtk-theme =
    { pkgs, config, ... }:
    {
      home.pointerCursor = {
        gtk.enable = true;
        x11.enable = true;
        size = 24;
      };

      dconf = {
        enable = true;
        settings = {
          "org/gnome/desktop/interface" = {
            color-scheme = "prefer-dark";
          };
        };
      };

      gtk = {
        enable = true;
        font = {
          name = config.var.fonts.sans;
          size = 11.5;
        };
        theme = {
          name = "Colloid-Dark-Compact-Catppuccin";
          package = pkgs.colloid-gtk-theme.override {
            colorVariants = [ "dark" ];
            themeVariants = [ "default" ];
            sizeVariants = [ "compact" ];
            tweaks = [
              "catppuccin"
              "rimless"
              "float"
            ];
          };
        };
      };

      qt = {
        enable = true;
        platformTheme.name = "gtk3";
        style.name = "kvantum";
      };
    };
}
