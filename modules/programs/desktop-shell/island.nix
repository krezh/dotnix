{
  flake.modules.homeManager.desktop-shell =
    { config, lib, ... }:
    let
      dusk = lib.getExe config.homeModules.dusk.package;
    in
    {
      homeModules.island = {
        enable = true;
        appearance = {
          opacity = config.var.opacity;
          radius = config.var.rounding;
          border = 1;
        };
        bar = {
          left = [
            "molt"
            "ai"
            "headset"
            "zowie"
            "media"
          ];
          middle = [ "workspaces" ];
          right = [
            "tray"
            "caffeine"
            "network"
            "bluetooth"
            "audio"
            "clock"
            "notifications"
            "session"
          ];
          spacing = 0;
        };

        features = {
          ai = {
            disabledLimits.omp.google-antigravity = [ "3p-weekly:weekly" ];
            mascot = "lively";
          };
          session = {
            confirm = false;
            commands = {
              logout = [
                dusk
                "logout"
              ];
              reboot = [
                dusk
                "reboot"
              ];
              shutdown = [
                dusk
                "shutdown"
              ];
            };
          };
        };
      };
    };
}
