{ pkgs, ... }:
{
  programs.kitty = {
    enable = true;
    package = pkgs.kitty;
    # https://sw.kovidgoyal.net/kitty/conf/
    settings = {
      cursor_trail = 1;
    };
  };
}
