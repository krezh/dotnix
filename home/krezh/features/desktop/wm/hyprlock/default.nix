{ inputs, pkgs, ... }:
{
  programs.hyprlock = {
    enable = true;
    package = inputs.hyprlock.packages.${pkgs.system}.hyprlock;
    settings = {
      general = {
        disable_loading_bar = true;
        grace = 3; # in seconds
        hide_cursor = true;
        no_fade_in = false;
      };

      background = [
        {
          monitor = "eDP-1";
          path = "screenshot";
          blur_passes = 3;
          blur_size = 8;
        }
      ];

      label = {
        monitor = "eDP-1";
        text = "$TIME";
        text_align = "center"; # center/right or any value for default left. multi-line text alignment inside label container
        color = "rgba(200, 200, 200, 1.0)";
        font_size = 50;
        font_family = "Noto Sans";
        rotate = 0; # degrees, counter-clockwise
        position = "0, 80"; # x, y
        halign = "center";
        valign = "center";
      };

      input-field = [
        {
          monitor = "eDP-1";
          size = "200, 50";
          position = "0, -80";
          dots_center = true;
          fade_on_empty = true;
          font_color = "rgb(202, 211, 245)";
          inner_color = "rgb(91, 96, 120)";
          outer_color = "rgb(24, 25, 38)";
          outline_thickness = 2;
          placeholder_text = ''<span foreground="##cad3f5"></span>'';
          shadow_passes = 2;
        }
      ];
    };
  };
}
