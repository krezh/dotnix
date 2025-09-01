{ osConfig, ... }:
{
  wayland.windowManager.hyprland.settings.monitor =
    if osConfig.networking.hostName == "thor" then
      [
        "DP-1,2560x1440@239.97,0x0,1.0"
        "DP-2,2560x1440@144.0,2560x0,1.0"
      ]
    else if osConfig.networking.hostName == "odin" then
      [
        "eDP-1,1920x1080@60.0,0x0,1.0"
      ]
    else
      [ ", preferred, auto, 1" ];
}
