{ pkgs, ... }:
{
  programs.tray-tui = {
    enable = true;
    package = pkgs.tray-tui;
  };
}
