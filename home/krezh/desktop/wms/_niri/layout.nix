{ var, ... }:
{
  programs.niri.settings = {
    layout = {
      gaps = 10;
      center-focused-column = "never";

      preset-column-widths = [
        { proportion = 0.33333; }
        { proportion = 0.5; }
        { proportion = 0.66667; }
      ];

      default-column-width = {
        proportion = 0.5;
      };

      focus-ring = {
        width = var.borderSize;
        active.color = "#89b4faff";
        inactive.color = "#1e1e2eff";
      };

      border.enable = false;
    };

    cursor = {
      theme = "catppuccin-mocha-dark-cursors";
      size = 24;
    };

    screenshot-path = "~/Pictures/Screenshots/Screenshot-%Y-%m-%d-%H-%M-%S.png";

    overview = {
      zoom = 0.25;
    };
  };
}
