{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.homeModules.chomp;
      jsonFormat = pkgs.formats.json { };
    in
    {
      options.homeModules.chomp = {
        enable = lib.mkEnableOption "chomp screenshot and screen recording tool";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.chomp;
          description = "chomp derivation to use.";
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

              delay = lib.mkOption {
                type = lib.types.nullOr lib.types.ints.unsigned;
                default = null;
                description = "Delay before capturing, in milliseconds.";
              };

              video = lib.mkOption {
                type = lib.types.submodule {
                  options = {
                    maxFps = lib.mkOption {
                      type = lib.types.ints.positive;
                      default = 60;
                      description = "Frame rate ceiling for screen recordings.";
                    };

                    encodeResolution = lib.mkOption {
                      type = lib.types.str;
                      default = "";
                      example = "1920x1080";
                      description = "Encoder resolution for screen recordings. Empty records at the monitor's own resolution.";
                    };

                    bitrate = lib.mkOption {
                      type = lib.types.str;
                      default = "";
                      example = "15 MB";
                      description = ''
                        Encoder bitrate, in bytes per second as wl-screenrec expects it,
                        so "15 MB" is 120 Mbps. Empty derives one from the recorded area
                        and frame rate, which keeps quality steady across resolutions.
                      '';
                    };

                    codec = lib.mkOption {
                      type = lib.types.enum [
                        "auto"
                        "avc"
                        "hevc"
                        "vp8"
                        "vp9"
                        "av1"
                      ];
                      default = "auto";
                      description = "Video codec for screen recordings. At a given bitrate hevc holds up better in motion than avc.";
                    };
                  };
                };
                default = { };
              };
            };
          };
          default = { };
        };

        ocr = lib.mkOption {
          type = lib.types.submodule {
            options = {
              language = lib.mkOption {
                type = lib.types.str;
                default = "eng";
                description = "Tesseract language code, which must be present in the tesseract package's data.";
              };
            };
          };
          default = { };
        };

        tools = lib.mkOption {
          type = lib.types.submodule {
            options = {
              satty = lib.mkOption {
                type = lib.types.package;
                default = pkgs.satty;
                description = "satty package used for annotation (the --annotate flag).";
              };

              wlCopy = lib.mkOption {
                type = lib.types.package;
                default = pkgs.wl-clipboard;
                description = "Package providing wl-copy, used for every clipboard copy.";
              };

              wlScreenrec = lib.mkOption {
                type = lib.types.package;
                default = pkgs.wl-screenrec;
                description = "Package used to record the screen.";
              };
            };
          };
          default = { };
        };
      };

      config = lib.mkIf cfg.enable {
        home.packages = [ cfg.package ];

        xdg.configFile."chomp/config.json".source = jsonFormat.generate "chomp-config.json" {
          font = {
            inherit (cfg.font) family size weight;
          };
          border = {
            inherit (cfg.border) color thickness rounding;
          };
          display = {
            dim_opacity = cfg.display.dimOpacity;
            inherit (cfg.display) log;
          };
          upload = {
            zipline = {
              inherit (cfg.zipline) url token;
              use_original_name = cfg.zipline.useOriginalName;
            };
          };
          capture = {
            save_path = cfg.capture.savePath;
            video = {
              max_fps = cfg.capture.video.maxFps;
              encode_resolution = cfg.capture.video.encodeResolution;
              inherit (cfg.capture.video) bitrate codec;
            };
          }
          // lib.optionalAttrs (cfg.capture.delay != null) { inherit (cfg.capture) delay; };
          ocr = {
            inherit (cfg.ocr) language;
          };
          tools = {
            satty = lib.getExe cfg.tools.satty;
            wl_copy = lib.getExe' cfg.tools.wlCopy "wl-copy";
            wl_screenrec = lib.getExe cfg.tools.wlScreenrec;
          };
        };
      };
    };
}
