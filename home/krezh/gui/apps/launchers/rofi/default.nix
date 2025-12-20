{ pkgs, ... }:
{
  catppuccin.rofi.enable = false;
  programs.rofi = {
    enable = true;
    package = pkgs.rofi;
    theme = ./outer.rasi;
    extraConfig = {
      show-icons = true;
    };
    plugins = with pkgs; [
      rofi-games
    ];
  };
  xdg.dataFile = {
    "rofi/themes/searchicon_w.svg" = {
      source = ./searchicon_w.svg;
    };
  };
}
