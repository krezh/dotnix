{ pkgs, ... }:
{
  home.packages = with pkgs; [
    vipsdisp
  ];
}
