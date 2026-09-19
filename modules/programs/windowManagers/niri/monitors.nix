{
  flake.modules.homeManager.niri =
    { osConfig, ... }:
    let
      outputs =
        if osConfig.networking.hostName == "thor" then
          [
            {
              output = {
                _args = [ "DP-1" ];
                mode = "2560x1440@239.97";
                scale = 1.0;
                position._props = {
                  x = 0;
                  y = 0;
                };
                variable-refresh-rate._props = {
                  on-demand = true;
                };
              };
            }
            {
              output = {
                _args = [ "DP-2" ];
                mode = "2560x1440@144";
                scale = 1.0;
                position._props = {
                  x = 2560;
                  y = 0;
                };
              };
            }
          ]
        else if osConfig.networking.hostName == "odin" then
          [
            {
              output = {
                _args = [ "eDP-1" ];
                mode = "1920x1080@60.0";
                scale = 1.0;
                position._props = {
                  x = 0;
                  y = 0;
                };
              };
            }
          ]
        else
          [ ];
    in
    {
      wayland.windowManager.niri.settings._children = outputs;
    };
}
