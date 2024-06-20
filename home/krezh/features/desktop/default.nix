{
  imports = [
    ./wm
    ./gtk
    ./apps
  ];
  # Import wallpapers into $HOME/wallpapers
  home.file."wallpapers" = {
    recursive = true;
    source = ./wallpapers;
  };
  fonts.fontconfig.enable = true;
}
