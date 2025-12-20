{ ... }:
{
  programs.foot = {
    enable = true;
    server.enable = true;
    settings = {
      main = {
        term = "xterm-256color";
        font = "JetbrainsMono Nerd Font:size=12";
        selection-target = "both";
      };
      scrollback = {
        lines = 10000;
        multiplier = 5;
      };
    };
  };
}
