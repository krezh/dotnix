{ pkgs, ... }:
{
  home.packages = with pkgs; [
    vdhcoapp
    antares
    wowup-cf
    yubikey-manager
    qbittorrent
  ];
}
