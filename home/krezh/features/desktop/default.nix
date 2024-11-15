{ ... }:
{
  imports = [
    ./wm
    ./gtk
    ./apps
    ./runners
  ];

  # Import wallpapers into $HOME/wallpapers
  home.file."wallpapers" = {
    recursive = true;
    source = ./wallpapers;
  };

  dconf.settings = {
    "org/gnome/desktop/interface" = {
      color-scheme = "prefer-dark";
    };
  };

  hmModules.desktop.upower-notify.enable = true;
}
