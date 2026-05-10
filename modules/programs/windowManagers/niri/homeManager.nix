{ inputs, ... }:
{
  flake.modules.homeManager.niri =
    {
      pkgs,
      config,
      ...
    }:
    {
      imports = [
        inputs.niri.homeModules.niri
      ];

      programs.niri.settings = {
        prefer-no-csd = true;
        clipboard.disable-primary = true;
        hotkey-overlay.skip-at-startup = true;
        gestures.hot-corners.enable = false;

        xwayland-satellite = {
          enable = true;
          path = "${pkgs.xwayland-satellite}/bin/xwayland-satellite";
        };

        spawn-at-startup = [
          {
            argv = [
              "dbus-update-activation-environment"
              "--systemd"
              "--all"
            ];
          }
        ];

        input = {
          keyboard.xkb.layout = "se";
          touchpad = {
            tap = true;
            dwt = true;
            dwtp = true;
            natural-scroll = true;
            accel-profile = "flat";
            accel-speed = 0.4;
          };
          mouse = {
            accel-profile = "flat";
            accel-speed = 0.4;
          };
        };

        outputs = {
          "DP-1" = {
            mode = {
              width = 2560;
              height = 1440;
              refresh = 239.970;
            };
            position = {
              x = 0;
              y = 0;
            };
            scale = 1;
          };
          "DP-2" = {
            mode = {
              width = 2560;
              height = 1440;
              refresh = 143.998;
            };
            position = {
              x = 2560;
              y = 0;
            };
            scale = 1;
          };
        };

        workspaces = {
          "a-1" = {
            name = "1";
            open-on-output = "DP-1";
          };
          "a-2" = {
            name = "2";
            open-on-output = "DP-1";
          };
          "a-3" = {
            name = "3";
            open-on-output = "DP-1";
          };
          "a-4" = {
            name = "4";
            open-on-output = "DP-2";
          };
          "a-5" = {
            name = "5";
            open-on-output = "DP-2";
          };
          "a-6" = {
            name = "6";
            open-on-output = "DP-2";
          };
        };

        layout = {
          gaps = 10;
          focus-ring = {
            width = config.var.borderSize;
            active.color = "#89b4faff";
            inactive.color = "#1e1e2eff";
          };
          border.enable = false;
        };

        cursor = {
          theme = "catppuccin-mocha-dark-cursors";
          size = 24;
        };
      };

      home.packages = with pkgs; [
        brightnessctl
        grim
        slurp
        wl-clipboard
        wl-screenrec
        swaylock
        swayidle
        xwayland-satellite
      ];

      home.sessionVariables = {
        NIXOS_OZONE_WL = "1";
        MOZ_ENABLE_WAYLAND = "1";
        QT_QPA_PLATFORM = "wayland";
        QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
        SDL_VIDEODRIVER = "wayland";
        _JAVA_AWT_WM_NONREPARENTING = "1";
      };
    };
}
