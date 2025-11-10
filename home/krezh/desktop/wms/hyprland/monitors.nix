{ osConfig, ... }:
{
  wayland.windowManager.hyprland.settings.monitorv2 =
    if osConfig.networking.hostName == "thor" then
      [
        {
          output = "DP-1";
          mode = "2560x1440@239.97";
          position = "0x0";
          scale = 1.0;
          bitdepth = 10;
          vrr = 2;
        }
        {
          output = "DP-2";
          mode = "2560x1440@144";
          position = "2560x0";
          scale = 1.0;
        }
      ]
    else if osConfig.networking.hostName == "odin" then
      [
        {
          output = "eDP-1";
          mode = "1920x1080@60.0";
          position = "0x0";
          scale = 1;
        }
      ]
    else
      [
        {
          output = "";
          mode = "preferred";
          position = "auto";
          scale = 1;
        }
      ];
}
