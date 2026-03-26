{
  flake.modules.homeManager.hyprland =
    { pkgs, config, ... }:
    {
      xdg.portal = {
        enable = true;
        extraPortals = with pkgs; [
          xdg-desktop-portal-hyprland
          xdg-desktop-portal-gtk
        ];
        # xdgOpenUsePortal = true;
        configPackages = [ config.wayland.windowManager.hyprland.package ];
        config.hyprland = {
          default = [
            "hyprland"
            "gtk"
          ];
          "org.freedesktop.impl.portal.FileChooser" = "gtk";
          "org.freedesktop.impl.portal.Print" = "gtk";
        };
      };
      wayland.windowManager.hyprland = {
        systemd = {
          enableXdgAutostart = true;
        };
      };

      # XDPH config
      xdg.configFile."hypr/xdph.conf".text = ''
        screencopy {
          max_fps = 120
          allow_token_by_default = true
        }
      '';
    };
}
