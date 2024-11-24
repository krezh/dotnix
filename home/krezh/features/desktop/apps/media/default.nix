{ pkgs, ... }:
{
  imports = [
    ./spotify
  ];

  home.packages = with pkgs; [
    plex-desktop
  ];
}
