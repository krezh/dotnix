{ ... }:
{
  programs.hyprlock = {
    enable = true;
    settings = {
      general = {
        grace = 3; # in seconds
        hide_cursor = true;
        no_fade_in = false;
        ignore_empty_input = true;
      };

      background = [
        {
          monitor = "";
          path = "screenshot";
          blur_passes = 4;
          blur_size = 8;
        }
      ];
    };
  };
}
