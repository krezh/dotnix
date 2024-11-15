{ pkgs, ... }:
{
  programs.kitty = {
    enable = true;
    package = pkgs.kitty;
    #extraConfig = builtins.readFile ./wezterm.lua;
  };
}
