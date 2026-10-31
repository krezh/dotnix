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
        font = config.var.fonts.sans;
      };
    };
}
