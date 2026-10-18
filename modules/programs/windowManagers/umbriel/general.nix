{ inputs, ... }:
{
  flake.modules.nixos.umbriel = {
    imports = [ inputs.umbriel.nixosModules.default ];
    programs.umbriel.enable = true;
  };

  flake.modules.homeManager.umbriel =
    {
      config,
      pkgs,
      ...
    }:
    {
      imports = [ inputs.umbriel.homeModules.default ];

      services.polkit-gnome.enable = true;

      programs.umbriel = {
        enable = true;
        settings = {
          general.show_cheatsheet = false;

          environment = {
            QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
            QT_QPA_PLATFORM = "wayland";
            NIXOS_OZONE_WL = "1";
          };

          input = {
            keyboard = {
              layout = "se";
              variant = "nodeadkeys";
              numlock_toggle = true;
            };
            touchpad = {
              tap = true;
              accel_profile = "flat";
              sensitivity = 0.4;
            };
            mouse = {
              accel_profile = "flat";
              sensitivity = 0.4;
            };
            focus.follows_mouse = true;
          };

          layout = {
            gap = 10;
            extent_presets = [
              0.33333
              0.5
              0.66667
              1.0
            ];
            scrolling = {
              center_focused = "never";
              default_extent_fraction = 0.5;
            };
          };

          appearance = {
            prefer_no_csd = true;
            border_width = config.var.borderSize;
            corner_radius = config.var.rounding;
            shadow = {
              enabled = true;
              softness = 40;
              offset_x = 4;
              offset_y = 4;
            };
          };

          colors = {
            shadow = "#00000030";
            border = {
              focused = "#89b4fa";
              unfocused = "#1e1e2e00";
            };
          };

          animation = {
            workspaces.curve = "spring:1,1000";
            windows_move.curve = "spring:1,800";
            windows_resize.curve = "spring:1,800";
            windows_in = {
              duration_ms = 200;
              curve = "easeout";
            };
            windows_out = {
              duration_ms = 150;
              curve = "easeout";
            };
          };
        };
      };

      home.packages = [ pkgs.tray-tui ];
    };
}
