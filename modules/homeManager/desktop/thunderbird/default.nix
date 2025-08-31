{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.thunderbird;
in
{
  options.hmModules.desktop.thunderbird = {
    enable = lib.mkEnableOption "thunderbird";

    package = lib.mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.thunderbird;
    };

    birdtray.enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable Birdtray, a system tray notification icon for Thunderbird.";
    };

    birdtray.package = lib.mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.birdtray;
      description = "The Birdtray package to use.";
    };

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = { };
    };
  };

  config = lib.mkIf cfg.enable {
    home = {
      packages = [
        cfg.package
        cfg.birdtray.package
      ];
    };

    systemd.user.services.birdtray = {
      Unit = {
        Description = "A new mail system tray notification icon for Thunderbird.";
        Documentation = "https://github.com/gyunaev/birdtray";
      };
      Service = {
        PassEnvironment = [
          "PATH"
          "XDG_RUNTIME_DIR"
        ];
        ExecStart = "${lib.getExe cfg.birdtray.package}";
        Restart = "if-failed";
        RestartSec = "5s";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
