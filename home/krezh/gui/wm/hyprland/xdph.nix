{ ... }:
{
  xdg.configFile = {
    "hypr/xdph.conf" = {
      text = ''
        screencopy {
            max_fps = 120
            allow_token_by_default = true;
        }
      '';
    };
  };
}
