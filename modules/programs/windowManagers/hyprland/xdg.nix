{ inputs, ... }:
{
  flake.modules.homeManager.hyprland =
    { pkgs, config, ... }:
    let
      picker = inputs.hyprland-preview-share-picker.packages.${pkgs.stdenv.hostPlatform.system}.default;
    in
    {
      home.packages = [ picker ];
      xdg.configFile."hyprland-preview-share-picker/config.yaml".text = builtins.toJSON {
        stylesheets = [ "theme.css" ];
        default_page = "windows";
        window = {
          width = 1100;
          height = 650;
        };
        image = {
          resize_size = 320;
          widget_size = 180;
        };
        windows = {
          min_per_row = 3;
          max_per_row = 4;
          clicks = 2;
          spacing = 12;
        };
        outputs = {
          clicks = 2;
          spacing = 8;
          show_label = true;
          respect_output_scaling = true;
        };
        hide_token_restore = false;
        debug = false;
      };

      xdg.configFile."hyprland-preview-share-picker/theme.css".text = ''
        @define-color picker_base #1e1e2e;
        @define-color picker_mantle #181825;
        @define-color picker_surface0 #313244;
        @define-color picker_surface1 #45475a;
        @define-color picker_overlay0 #6c7086;
        @define-color picker_text #cdd6f4;
        @define-color picker_subtext0 #a6adc8;
        @define-color picker_blue #89b4fa;
        @define-color picker_green #a6e3a1;

        * {
          font-family: "${config.var.fonts.sans}";
          color: @picker_text;
        }

        .window {
          margin: 4px;
          border: 2px solid @picker_blue;
          border-radius: ${toString config.var.rounding}px;
          background-color: @picker_base;
          box-shadow: 0 12px 36px alpha(@picker_mantle, 0.85);
        }

        .notebook,
        .notebook > stack {
          border: 0;
          background-color: transparent;
        }

        tabs {
          padding: 12px 16px 0;
          border: 0;
          background-color: @picker_mantle;
        }

        tabs > tab {
          margin: 0 6px;
          padding: 9px 14px 11px;
          border-bottom: 2px solid transparent;
          transition: 150ms ease;
        }

        tabs > tab:hover {
          border-bottom-color: @picker_surface1;
        }

        tabs > tab:checked,
        tabs > tab:active,
        tabs > tab:focus {
          border-bottom-color: @picker_blue;
        }

        .tab-label {
          color: @picker_subtext0;
          font-size: 14px;
          font-weight: 600;
        }

        tabs > tab:checked > .tab-label,
        tabs > tab:active > .tab-label,
        tabs > tab:focus > .tab-label {
          color: @picker_blue;
        }

        .page {
          padding: 18px;
          background-color: @picker_base;
        }

        flowboxchild,
        button {
          padding: 0;
          border: 0;
          background-color: transparent;
          background-image: none;
          box-shadow: none;
        }

        flowboxchild > .card,
        button > .card {
          padding: 8px;
          border: 2px solid @picker_surface1;
          border-radius: ${toString config.var.rounding}px;
          background-color: @picker_surface0;
          box-shadow: 0 6px 18px alpha(@picker_mantle, 0.65);
          transition: 150ms ease;
        }

        flowboxchild:hover > .card,
        button:hover > .card {
          border-color: @picker_overlay0;
          background-color: @picker_surface1;
        }

        flowboxchild:active > .card,
        flowboxchild:selected > .card,
        flowboxchild:focus > .card,
        button:active > .card,
        button:selected > .card,
        button:focus > .card {
          border-color: @picker_blue;
          box-shadow: 0 0 0 2px alpha(@picker_blue, 0.22);
        }

        .card-loading {
          opacity: 0.65;
        }

        .image {
          border-radius: ${toString (config.var.rounding - 5)}px;
          background-color: @picker_mantle;
        }

        .image-label {
          padding: 8px 6px 2px;
          color: @picker_text;
          font-size: 13px;
          font-weight: 500;
        }

        .region-button {
          padding: 12px 24px;
          border: 2px solid @picker_blue;
          border-radius: ${toString config.var.rounding}px;
          background-color: @picker_blue;
          color: @picker_base;
          font-size: 14px;
          font-weight: 700;
          box-shadow: 0 6px 18px alpha(@picker_mantle, 0.65);
          transition: 150ms ease;
        }

        .region-button:hover,
        .region-button:focus {
          border-color: @picker_green;
          background-color: @picker_green;
        }

        .region-button:active {
          background-color: shade(@picker_blue, 0.85);
        }

        .restore-button {
          margin: 0 18px 16px;
          padding: 8px 12px;
          border-radius: ${toString config.var.rounding}px;
          background-color: @picker_surface0;
          color: @picker_subtext0;
        }

        .restore-button:hover,
        .restore-button:focus {
          background-color: @picker_surface1;
          color: @picker_text;
        }

        .restore-button:checked {
          color: @picker_blue;
        }

        scrollbar slider {
          min-width: 6px;
          min-height: 6px;
          border-radius: 999px;
          background-color: @picker_surface1;
        }

        scrollbar slider:hover {
          background-color: @picker_blue;
        }
      '';

      xdg.portal = {
        enable = true;
        extraPortals = with pkgs; [
          xdg-desktop-portal-hyprland
          xdg-desktop-portal-gtk
        ];
        # xdgOpenUsePortal = true;
        configPackages = [ config.wayland.windowManager.hyprland.package ];
        config.hyprland = {
          default = [
            "hyprland"
            "gtk"
          ];
          "org.freedesktop.impl.portal.FileChooser" = "gtk";
          "org.freedesktop.impl.portal.Print" = "gtk";
        };
      };
      # XDPH config
      xdg.configFile."hypr/xdph.conf".text = ''
        screencopy {
          max_fps = 120
          custom_picker_binary = ${picker}/bin/hyprland-preview-share-picker
          allow_token_by_default = true
        }
      '';
    };
}
