{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.programs.gulp;
      jsonFormat = pkgs.formats.json { };
    in
    {
      options.programs.gulp = {
        enable = lib.mkEnableOption "gulp screenshot and screen recording tool";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.gulp;
          description = "gulp derivation to use.";
        };

        font = lib.mkOption {
          type = lib.types.submodule {
            options = {
              family = lib.mkOption {
                type = lib.types.str;
                default = "Inter";
                description = "Font family for dimension text overlay.";
              };

              size = lib.mkOption {
                type = lib.types.ints.positive;
                default = 16;
                description = "Font size for dimension text overlay.";
              };

              weight = lib.mkOption {
                type = lib.types.enum [
                  "Normal"
                  "Bold"
                ];
                default = "Bold";
                description = "Font weight for dimension text overlay.";
              };
            };
          };
          default = { };
        };

        border = lib.mkOption {
          type = lib.types.submodule {
            options = {
              color = lib.mkOption {
                type = lib.types.str;
                default = "#FFFFFF";
                description = "Border color in hex format (e.g., #FFFFFF).";
              };

              thickness = lib.mkOption {
                type = lib.types.ints.unsigned;
                default = 2;
                description = "Border thickness in pixels.";
              };

              rounding = lib.mkOption {
                type = lib.types.ints.unsigned;
                default = 0;
                description = "Border corner rounding in pixels.";
              };
            };
          };
          default = { };
        };

        display = lib.mkOption {
          type = lib.types.submodule {
            options = {
              dimOpacity = lib.mkOption {
                type = lib.types.float;
                default = 0.5;
                description = "Opacity of the dimmed overlay (0.0 to 1.0).";
              };

              fps = lib.mkOption {
                type = lib.types.ints.unsigned;
                default = 0;
                description = "Maximum frames per second for selection overlay (0 = auto-detect from monitor).";
              };

              log = lib.mkOption {
                type = lib.types.enum [
                  "off"
                  "info"
                  "debug"
                  "warn"
                  "error"
                ];
                default = "off";
                description = "Logging level.";
              };
            };
          };
          default = { };
        };

        zipline = lib.mkOption {
          type = lib.types.submodule {
            options = {
              url = lib.mkOption {
                type = lib.types.str;
                default = "";
                description = "Zipline server URL for automatic uploads.";
              };

              token = lib.mkOption {
                type = lib.types.str;
                default = "";
                description = "Path to Zipline authentication token file.";
              };

              useOriginalName = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Use original filename when uploading to Zipline.";
              };
            };
          };
          default = { };
        };

        capture = lib.mkOption {
          type = lib.types.submodule {
            options = {
              savePath = lib.mkOption {
                type = lib.types.str;
                default = "/tmp";
                description = "Default directory for saving screenshots and recordings.";
              };
            };
          };
          default = { };
        };
      };

      config = lib.mkIf cfg.enable {
        home.packages = [ cfg.package ];

        xdg.configFile."gulp/config.json".source = jsonFormat.generate "gulp-config.json" {
          font = {
            family = cfg.font.family;
            size = cfg.font.size;
            weight = cfg.font.weight;
          };
          border = {
            color = cfg.border.color;
            thickness = cfg.border.thickness;
            rounding = cfg.border.rounding;
          };
          display = {
            dim_opacity = cfg.display.dimOpacity;
            fps = cfg.display.fps;
            log = cfg.display.log;
          };
          upload = {
            zipline = {
              url = cfg.zipline.url;
              token = cfg.zipline.token;
              use_original_name = cfg.zipline.useOriginalName;
            };
          };
          capture = {
            save_path = cfg.capture.savePath;
          };
        };
      };
    };
}
