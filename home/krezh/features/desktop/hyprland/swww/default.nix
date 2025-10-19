{
  config,
  ...
}:
{
  hmModules.desktop.swww = {
    enable = true;
    # package = inputs.swww.packages.${pkgs.system}.swww;
    path = config.home.file.wallpapers.source;
    settings.interval = 600; # 10 minutes
  };
}
