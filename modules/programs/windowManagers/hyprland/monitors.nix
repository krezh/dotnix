{
  flake.modules.homeManager.hyprland =
    { osConfig, lib, ... }:
    let
      inherit (import ./_helpers.nix { inherit lib; }) mkMonitors;
    in
    {
      wayland.windowManager.hyprland = {
        settings = {
          # "<output>" = "mode, position, scale" — or an attrset for extra keys.
          monitor =
            if osConfig.networking.hostName == "thor" then
              mkMonitors {
                # vrr = 2 (fullscreen-only) pinned the OTG to vmax/vmin 16364 whenever a
                # window went fullscreen, driving the panel at ~24 Hz instead of 240.
                "DP-1" = {
                  mode = "2560x1440@239.97";
                  position = "0x0";
                  scale = 1.0;
                  vrr = 0;
                };
                "DP-2" = "2560x1440@144, 2560x0, 1.0";
              }
            else if osConfig.networking.hostName == "odin" then
              mkMonitors { "eDP-1" = "1920x1080@60.0, 0x0, 1"; }
            else
              mkMonitors { "" = "preferred, auto, 1"; };
        };
      };
    };
}
