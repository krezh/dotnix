{ pkgs, ... }:
{
  home.packages = with pkgs; [
    gnome-clocks
    mpris-timer
    gnome-calculator
    gnome-calendar
    gnome-bluetooth
    gnome-maps
    gnome-online-accounts-gtk
    geary
  ];
}
