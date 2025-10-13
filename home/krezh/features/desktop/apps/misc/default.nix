{ pkgs, ... }:
{
  imports = [
    ./kde-connect
    ./file-roller
    ./bitwarden
  ];

  home.packages = with pkgs; [
    gnome-clocks
    speedtest-cli
  ];
}
