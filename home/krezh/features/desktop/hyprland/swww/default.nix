{
  config,
  pkgs,
  inputs,
  ...
}:
{
  hmModules.desktop.swww = {
    enable = true;
    package = inputs.swww.packages.${pkgs.system}.swww;
    path = config.home.file.wallpapers.source;
  };
}
