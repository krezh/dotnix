{
  flake.modules.homeManager.desktop-shell =
    { config, ... }:
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
            "media"
            "tray"
          ];
          middle = [ "workspaces" ];
          right = [
            "caffeine"
            "network"
            "bluetooth"
            "audio"
            "headset"
            "zowie"
            "clock"
            "notifications"
            "session"
          ];
          spacing = 0;
          group = [
            "network"
            "bluetooth"
            "audio"
            "headset"
            "zowie"
          ];
        };

        features = {
          ai = {
            mascot = "lively";
          };
          gotify = {
            enable = true;
            tokenFile = config.sops.secrets."gotify/token".path;
            url = "wss://gotify.plexuz.xyz";
          };
          session.confirm = false;
        };
      };
    };
}
