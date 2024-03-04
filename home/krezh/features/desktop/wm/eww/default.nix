{ pkgs, ... }: {
  programs.eww = {
    enable = true;
    package = pkgs.eww-wayland;
    configDir = ./eww;
  };

  home.packages = [ pkgs.xclip pkgs.wmctrl pkgs.light pkgs.brightnessctl ];
}
