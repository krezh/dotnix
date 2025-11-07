{ ... }:
{
  # Import wallpapers into $HOME/wallpapers
  home.file."wallpapers" = {
    recursive = true;
    source = ./wallpapers;
  };

  services.udiskie = {
    enable = true;
  };
}
