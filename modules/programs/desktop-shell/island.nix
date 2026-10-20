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
            "audio"
            "clock"
            "notifications"
            "session"
          ];
          spacing = 0;
        };
        features = {
          ai.disabledLimits.omp.google-antigravity = [ "3p-weekly:weekly" ];
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
