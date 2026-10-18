{
  flake.modules.homeManager.umbriel =
    { osConfig, ... }:
    {
      programs.umbriel.settings.output =
        if osConfig.networking.hostName == "thor" then
          {
            "DP-1" = {
              mode = "2560x1440@239.97";
              scale = 1.0;
              position = [
                0
                0
              ];
            };
            "DP-2" = {
              mode = "2560x1440@144";
              scale = 1.0;
              position = [
                2560
                0
              ];
            };
          }
        else if osConfig.networking.hostName == "odin" then
          {
            "eDP-1" = {
              mode = "1920x1080@60.0";
              scale = 1.0;
              position = [
                0
                0
              ];
            };
          }
        else
          { };
    };
}
