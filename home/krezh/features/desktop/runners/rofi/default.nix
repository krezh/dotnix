{ pkgs, ... }:
{
  catppuccin.rofi.enable = false;
  programs.rofi = {
    enable = true;
    package = pkgs.rofi;
    theme = {
      "@theme" = "catppuccin-default";
      "@import" = "catppuccin";
    };
    extraConfig = {
      show-icons = true;
      display-ssh = "󰣀 ssh:";
      display-run = "󱓞 run:";
      display-drun = "󰣖 drun:";
      display-window = "󱬀 window:";
      display-combi = "󰕘 combi:";
      display-filebrowser = "󰉋 filebrowser:";
    };

  };
  xdg.dataFile = {
    "rofi/themes/catppuccin-default.rasi" = {
      source = ./catppuccin-lavrent-mocha.rasi;
    };
    "rofi/themes/catppuccin.rasi" = {
      source = ./catppuccin-lavrent-mocha.rasi;
    };
  };
}
