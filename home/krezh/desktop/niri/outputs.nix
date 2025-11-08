{ ... }:
{
  programs.niri.settings.outputs = {
    "DP-1" = {
      mode = {
        width = 2560;
        height = 1440;
        refresh = 239.970;
      };
      position = {
        x = 0;
        y = 0;
      };
      variable-refresh-rate = true;
    };

    "DP-2" = {
      mode = {
        width = 2560;
        height = 1440;
        refresh = 143.998;
      };
      position = {
        x = 2560;
        y = 0;
      };
      variable-refresh-rate = true;
    };
  };
}
