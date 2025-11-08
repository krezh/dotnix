{
  config,
  ...
}:
{
  hmModules.desktop.swww = {
    enable = true;
    path = config.home.file.wallpapers.source;
    settings.interval = 600; # 10 minutes
  };
}
