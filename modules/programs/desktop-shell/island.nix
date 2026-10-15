{
  flake.modules.homeManager.desktop-shell =
    { config, ... }:
    {
      homeModules.island = {
        enable = true;
        opacity = config.var.opacity;
        radius = config.var.rounding;
      };
    };
}
