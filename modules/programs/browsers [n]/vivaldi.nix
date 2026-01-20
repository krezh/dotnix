{
  flake.modules.homeManager.browsers = {pkgs, ...}: {
    home.packages = [pkgs.vivaldi];
  };
}
