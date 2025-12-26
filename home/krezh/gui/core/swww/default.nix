{
  config,
  pkgs,
  ...
}:
{
  hmModules.desktop.swww = {
    enable = true;
    path = config.home.file.wallpapers.source;
    settings.interval = 60 * 10; # 10 minutes
  };

  # Import wallpapers into $HOME/wallpapers
  home.file."wallpapers" = {
    recursive = true;
    source = ../wallpapers;
  };

  home.packages = [ pkgs.waypaper ];
}
