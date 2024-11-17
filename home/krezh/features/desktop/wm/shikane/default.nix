{ ... }:
{
  hmModules.desktop.shikane = {
    enable = false;
    config = {
      profile = [
        {
          name = "default";
          output = [
            {
              search = [ "eDP-1" ];
              mode = "1920x1080@60";
              enable = true;
            }
          ];
        }
        {
          name = "docking";
          exec = [ "notify-send shikane profile $SHIKANE_PROFILE_NAME active" ];
          output = [
            {
              search = [ "eDP-1" ];
              enable = false;
            }
            {
              search = [ "DP-[1-9]" ];
              enable = true;
            }
          ];
        }
      ];
    };
  };
}
