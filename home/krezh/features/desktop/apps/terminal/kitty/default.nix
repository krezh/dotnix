{ pkgs, ... }:
{
  programs.kitty = {
    enable = true;
    package = pkgs.kitty;
    settings = {
      cursor_trail = 1;
    };
  };
}
