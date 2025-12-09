{ ... }:
{
  programs.niri.settings = {
    prefer-no-csd = true;
    clipboard = {
      disable-primary = true;
    };
    hotkey-overlay = {
      skip-at-startup = true;
    };
    gestures = {
      hot-corners.enable = false;
    };
  };
}
