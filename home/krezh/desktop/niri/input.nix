{ ... }:
{
  programs.niri.settings.input = {
    keyboard = {
      xkb = {
        layout = "se";
      };
      repeat-delay = 600;
      repeat-rate = 25;
    };

    touchpad = {
      tap = true;
      dwt = true;
      dwtp = true;
      natural-scroll = true;
      accel-profile = "flat";
      accel-speed = 0.4;
    };

    mouse = {
      accel-profile = "flat";
      accel-speed = 0.4;
    };
  };
}
