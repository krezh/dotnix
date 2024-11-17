{ ... }:
{
  hmModules.desktop.shikane = {
    enable = true;
    config = {
      profile = [
        {
          name = "default";
          output = [
            {
              name = "eDP-1";
              enable = true;
            }
          ];
        }
        {
          name = "docking";
          exec = [ ''notify-send "shikane" "profile $SHIKANE_PROFILE_NAME active"'' ];
          output = [
            {
              match = "eDP-1";
              enable = false;
            }
            {
              match = "/DP-[1-9]/";
              enable = true;
            }
          ];
        }
      ];
    };
  };
}
