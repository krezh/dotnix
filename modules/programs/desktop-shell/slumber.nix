{
  flake.modules.homeManager.desktop-shell =
    { config, ... }:
    {
      homeModules.slumber = {
        enable = true;
        animation = "waves";
        showClock = true;
        spanOutputs = true;
        opacity = 0.0;
        blur = config.var.blur;
        font = config.var.fonts.sans;
      };
    };
}
