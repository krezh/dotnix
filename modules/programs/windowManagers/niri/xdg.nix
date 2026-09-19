{
  flake.modules.homeManager.niri =
    { pkgs, config, ... }:
    {
      xdg.portal = {
        enable = true;
        extraPortals = with pkgs; [
          xdg-desktop-portal-gnome
          xdg-desktop-portal-gtk
        ];
        configPackages = [ config.wayland.windowManager.niri.package ];
        config.niri = {
          default = [
            "gnome"
            "gtk"
          ];
          "org.freedesktop.impl.portal.FileChooser" = "gtk";
          "org.freedesktop.impl.portal.Print" = "gtk";
        };
      };
    };
}
