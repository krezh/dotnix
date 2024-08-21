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

  hmModules.desktop.upower-notify.enable = true;
}
