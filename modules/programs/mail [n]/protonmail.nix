{
  flake.modules.homeManager.mail = {pkgs, ...}: {
    home.packages = [pkgs.protonmail-desktop];
  };
}
