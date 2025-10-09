{ pkgs, ... }:
{
  home.packages = with pkgs; [
    curlie
  ];
}
