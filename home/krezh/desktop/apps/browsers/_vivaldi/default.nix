{ pkgs, ... }:
{
  programs = {
    chromium.enable = false;
    chromium.package = pkgs.vivaldi;
    chromium.commandLineArgs = [
      "--enable-blink-features=MiddleClickAutoscroll"
    ];
  };
}
