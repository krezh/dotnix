{ pkgs, ... }:
{
  home.packages = with pkgs; [
    evince
    gnome-clocks
  ];
}
