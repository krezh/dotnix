{
  flake.modules.homeManager.desktop-utils =
    { pkgs, ... }:
    {
      home.packages = [ pkgs.yapfast ];
    };
}
