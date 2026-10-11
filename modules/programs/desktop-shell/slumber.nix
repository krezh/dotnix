{
  flake.modules.homeManager.desktop-shell =
    {
      homeModules.slumber = {
        enable = true;
        animation = "waves";
        showClock = true;
        opacity = 0.75;
        font = "Rubik";
      };
    };
}
