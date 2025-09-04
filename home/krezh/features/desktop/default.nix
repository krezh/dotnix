{ ... }:
{
  imports = [
    ./hyprland
    ./gtk
    ./apps
    ./runners
    ./xdg
  ];

  # Import wallpapers into $HOME/wallpapers
  home.file."wallpapers" = {
    recursive = true;
    source = ./wallpapers;
  };

  hmModules.desktop.upower-notify.enable = false;
  services.udiskie = {
    enable = true;
  };
}
