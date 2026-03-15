{
  flake.modules.homeManager.browsers =
    { pkgs, ... }:
    {
      programs.chromium.enable = true;
      programs.chromium.package = pkgs.vivaldi;
    };
}
