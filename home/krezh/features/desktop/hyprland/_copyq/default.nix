{ pkgs, ... }:
{
  services.copyq = {
    enable = true;
    systemdTarget = "hyprland-session.target";
    forceXWayland = false;
  };

  xdg.configFile."copyq/themes/catppuccin-mocha.ini" = {
    source = ./catppuccin-mocha.ini;
  };

  home.packages = [ pkgs.wl-clipboard ];
}
