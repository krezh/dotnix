{ config, ... }:
{
  modules.desktop.swww = {
    enable = true;
    interval = 5 * 60;
    path = "${config.home.file.wallpapers.source}";
  };
}
