{ pkgs, ... }:
{
  home.packages = with pkgs; [
    vdhcoapp
    antares
    gulp
    wowup-cf
    yubikey-manager
    qbittorrent
  ];
}
