{ pkgs, ... }:
{
  home.packages = with pkgs; [
    vdhcoapp
    wowup-cf
    yubikey-manager
    qbittorrent
    sqlit-tui
  ];
}
