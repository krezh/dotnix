{ pkgs, ... }:
{
  imports = [ ];

  home.packages = [
    pkgs.walker
  ];
}
