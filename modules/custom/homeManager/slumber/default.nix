{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.homeModules.slumber;
      tomlFormat = pkgs.formats.toml { };
    in
    {
      options.homeModules.slumber = {
        enable = lib.mkEnableOption "slumber Wayland screensaver";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.slumber;
          description = "slumber package to use.";
        };

        animation = lib.mkOption {
          type = lib.types.enum [
            "random"
            "waves"
            "starfield"
            "clock"
          ];
          default = "random";
          description = "Default animation to display.";
        };

        theme = lib.mkOption {
          type = lib.types.enum [
            "mocha"
            "macchiato"
            "frappe"
            "latte"
          ];
          default = "mocha";
          description = "Catppuccin theme flavor.";
        };

        fps = lib.mkOption {
          type = lib.types.ints.positive;
          default = 60;
          description = "Target frames per second.";
        };

        motionThreshold = lib.mkOption {
          type = lib.types.float;
          default = 5.0;
          description = "Mouse movement distance in pixels required to wake up.";
        };

        graceMs = lib.mkOption {
          type = lib.types.ints.unsigned;
          default = 200;
          description = "Milliseconds to ignore initial mouse motion after launch.";
        };

        cycle = lib.mkOption {
          type = lib.types.ints.unsigned;
          default = 0;
          description = "Cycle to a new animation every N seconds (0 = disabled).";
        };

        showClock = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Overlay the digital clock on top of the animation.";
        };

        opacity = lib.mkOption {
          type = lib.types.float;
          default = 1.0;
          description = "Background opacity from 0.0 (transparent) to 1.0 (opaque).";
        };

        font = lib.mkOption {
          type = lib.types.str;
          default = "Rubik";
          description = "Font family or path to font file for screensaver typography.";
        };

        speed = lib.mkOption {
          type = lib.types.float;
          default = 1.0;
          description = "Animation speed multiplier.";
        };
      };

      config = lib.mkIf cfg.enable {
        home.packages = [ cfg.package ];

        xdg.configFile."slumber/config.toml".source = tomlFormat.generate "slumber-config.toml" {
          animation = cfg.animation;
          theme = cfg.theme;
          fps = cfg.fps;
          motion_threshold = cfg.motionThreshold;
          grace_ms = cfg.graceMs;
          cycle = cfg.cycle;
          show_clock = cfg.showClock;
          opacity = cfg.opacity;
          font = cfg.font;
          speed = cfg.speed;
        };
      };
    };
}
