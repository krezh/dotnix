{
  flake.modules.homeManager.gtk-theme =
    { pkgs, config, ... }:
    {
      catppuccin.cursors = {
        enable = true;
        flavor = "mocha";
        accent = "light";
      };

      home.pointerCursor = {
        enable = true;
        gtk.enable = true;
        x11.enable = true;
        hyprcursor.enable = true;
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
        gtk4.theme = config.gtk.theme;
        font = {
          name = config.var.fonts.sans;
          size = 11.5;
        };
        theme = {
          name = "WhiteSur-Dark-blue";
          package = pkgs.whitesur-gtk-theme.override {
            colorVariants = [ "dark" ];
            themeVariants = [ "blue" ];
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
