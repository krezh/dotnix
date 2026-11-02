{
  flake.modules.homeManager.office =
    { pkgs, ... }:
    {
      home.packages = [ pkgs.libreoffice-qt6 ];
    };
}
