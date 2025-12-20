{ ... }:
{
  programs.alacritty = {
    enable = true;
    settings = {
      window = {
        padding = {
          y = 5;
          x = 5;
        };
      };

      font = {
        normal.family = "JetbrainsMono Nerd Font";
        size = 12.0;
      };
    };
  };
}
