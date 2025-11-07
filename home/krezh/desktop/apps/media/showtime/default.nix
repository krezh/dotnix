{ pkgs, ... }:
{
  home.packages = with pkgs; [
    showtime
  ];
}
