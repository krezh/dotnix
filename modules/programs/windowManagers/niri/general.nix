{
  flake.modules.nixos.niri = {
    programs.niri.enable = true;
  };

  flake.modules.homeManager.niri =
    {
      pkgs,
      config,
      ...
    }:
    {
      services.polkit-gnome.enable = true;

      wayland.windowManager.niri = {
        enable = true;
        systemd = {
          enable = true;
          variables = [ "--all" ];
        };

        settings = {
          prefer-no-csd = { };

          hotkey-overlay.skip-at-startup = { };

          input = {
            keyboard = {
              xkb = {
                layout = "se";
                variant = "nodeadkeys";
              };
              numlock = { };
            };

            touchpad = {
              tap = { };
              accel-profile = "flat";
              accel-speed = 0.4;
            };

            mouse = {
              accel-profile = "flat";
              accel-speed = 0.4;
            };

            focus-follows-mouse = { };
          };

          layout = {
            gaps = 10;
            center-focused-column = "never";
            default-column-width = {
              proportion = 0.5;
            };

            preset-column-widths._children = [
              { proportion = 0.33333; }
              { proportion = 0.5; }
              { proportion = 0.66667; }
              { proportion = 1.0; }
            ];

            focus-ring.off = { };

            border = {
              width = config.var.borderSize;
              active-gradient._props = {
                from = "#89b4fa";
                to = "#a6e3a1";
                angle = 125;
                relative-to = "workspace-view";
              };
              inactive-color = "#1e1e2e00";
            };

            shadow = {
              softness = 40;
              spread = 1;
              offset._props = {
                x = 4;
                y = 4;
              };
              color = "#00000030";
            };
          };

          animations = {
            workspace-switch = {
              spring._props = {
                damping-ratio = 1.0;
                stiffness = 1000;
                epsilon = 0.0001;
              };
            };
            horizontal-view-movement = {
              spring._props = {
                damping-ratio = 1.0;
                stiffness = 800;
                epsilon = 0.0001;
              };
            };
            window-open = {
              duration-ms = 200;
              curve = "ease-out-expo";
            };
            window-close = {
              duration-ms = 150;
              curve = "ease-out-quad";
            };
            window-movement = {
              spring._props = {
                damping-ratio = 1.0;
                stiffness = 800;
                epsilon = 0.0001;
              };
            };
          };

          environment = {
            QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
            QT_QPA_PLATFORM = "wayland";
            NIXOS_OZONE_WL = "1";
          };
        };
      };

      home.packages = [
        pkgs.xwayland-satellite
        pkgs.tray-tui
      ];
    };
}
