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
                  "trace"
                ];
                default = "off";
                description = "Logging level.";
              };

              freeze = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Freeze the screen while selecting a region.";
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

              replay = lib.mkOption {
                type = lib.types.submodule {
                  options = {
                    enabled = lib.mkEnableOption "the native instant replay service" // {
                      default = true;
                    };
                    durationSeconds = lib.mkOption {
                      type = lib.types.ints.between 5 600;
                      default = 30;
                      description = "Seconds of encoded game video retained in memory.";
                    };
                    retainAfterExitSeconds = lib.mkOption {
                      type = lib.types.ints.unsigned;
                      default = 120;
                      description = "Seconds the final replay remains available after a game exits.";
                    };
                    hyprlandTag = lib.mkOption {
                      type = lib.types.nonEmptyStr;
                      description = "Hyprland window tag selecting the game capture target.";
                    };
                    driDevice = lib.mkOption {
                      type = lib.types.str;
                      default = "/dev/dri/renderD128";
                      description = "DRM render node used by the VA-API encoder.";
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

        keybinds = lib.mkOption {
          type = lib.types.submodule {
            options = {
              screenshotArea = lib.mkOption {
                type = lib.types.str;
                default = "a";
              };
              screenshotScreen = lib.mkOption {
                type = lib.types.str;
                default = "s";
              };
              screenshotWindow = lib.mkOption {
                type = lib.types.str;
                default = "w";
              };
              ocr = lib.mkOption {
                type = lib.types.str;
                default = "c";
              };
              recordArea = lib.mkOption {
                type = lib.types.str;
                default = "A";
              };
              recordScreen = lib.mkOption {
                type = lib.types.str;
                default = "S";
              };
              recordWindow = lib.mkOption {
                type = lib.types.str;
                default = "W";
              };
              stopRecording = lib.mkOption {
                type = lib.types.str;
                default = "x";
              };
              replaySave = lib.mkOption {
                type = lib.types.str;
                default = "r";
              };
            };
          };
          default = { };
          description = "Single-character mode selector keybindings.";
        };

        modeSelect = lib.mkOption {
          type = lib.types.submodule {
            options = {
              backgroundColor = lib.mkOption {
                type = lib.types.str;
                default = "#0D0D14";
              };
              backgroundOpacity = lib.mkOption {
                type = lib.types.float;
                default = 0.95;
              };
              controlHeight = lib.mkOption {
                type = lib.types.ints.positive;
                default = 56;
              };
              borderOpacity = lib.mkOption {
                type = lib.types.float;
                default = 0.35;
              };
              keyColor = lib.mkOption {
                type = lib.types.str;
                default = "";
              };
              descriptionColor = lib.mkOption {
                type = lib.types.str;
                default = "#FFFFFF";
              };
              descriptionOpacity = lib.mkOption {
                type = lib.types.float;
                default = 0.85;
              };
              controlBorderOpacity = lib.mkOption {
                type = lib.types.float;
                default = 0.18;
              };
              recordingDotColor = lib.mkOption {
                type = lib.types.str;
                default = "#F24040";
              };
              recordingHighlightColor = lib.mkOption {
                type = lib.types.str;
                default = "#F2BF33";
              };
              replayColor = lib.mkOption {
                type = lib.types.str;
                default = "#38BDF8";
              };
            };
          };
          default = { };
          description = "Mode selector appearance.";
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
            inherit (cfg.display) freeze log;
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
            replay =
              {
                inherit (cfg.capture.replay) enabled;
                duration_seconds = cfg.capture.replay.durationSeconds;
                retain_after_exit_seconds = cfg.capture.replay.retainAfterExitSeconds;
                dri_device = cfg.capture.replay.driDevice;
              }
              // lib.optionalAttrs cfg.capture.replay.enabled {
                hyprland_tag = cfg.capture.replay.hyprlandTag;
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
          keybinds = {
            screenshot_area = cfg.keybinds.screenshotArea;
            screenshot_screen = cfg.keybinds.screenshotScreen;
            screenshot_window = cfg.keybinds.screenshotWindow;
            inherit (cfg.keybinds) ocr;
            record_area = cfg.keybinds.recordArea;
            record_screen = cfg.keybinds.recordScreen;
            record_window = cfg.keybinds.recordWindow;
            stop_recording = cfg.keybinds.stopRecording;
            replay_save = cfg.keybinds.replaySave;
          };
          mode_select = {
            background_color = cfg.modeSelect.backgroundColor;
            background_opacity = cfg.modeSelect.backgroundOpacity;
            control_height = cfg.modeSelect.controlHeight;
            border_opacity = cfg.modeSelect.borderOpacity;
            key_color = cfg.modeSelect.keyColor;
            description_color = cfg.modeSelect.descriptionColor;
            description_opacity = cfg.modeSelect.descriptionOpacity;
            control_border_opacity = cfg.modeSelect.controlBorderOpacity;
            recording_dot_color = cfg.modeSelect.recordingDotColor;
            recording_highlight_color = cfg.modeSelect.recordingHighlightColor;
            replay_color = cfg.modeSelect.replayColor;
          };
        };

        systemd.user.services.chomp-replay = lib.mkIf cfg.capture.replay.enabled {
          Unit = {
            Description = "Chomp instant replay service";
            PartOf = [ "graphical-session.target" ];
            After = [ "graphical-session.target" ];
          };
          Service = {
            ExecStart = "${lib.getExe cfg.package} --replay-service";
            Restart = "on-failure";
            RestartSec = 1;
            RuntimeDirectory = "chomp";
            RuntimeDirectoryMode = "0700";
          };
          Install.WantedBy = [ "graphical-session.target" ];
        };
      };
    };
}
