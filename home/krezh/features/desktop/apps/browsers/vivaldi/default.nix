{ pkgs, ... }:
{
  programs = {
    chromium.enable = true;
    chromium.package = pkgs.vivaldi;
    chromium.commandLineArgs = [
      "--enable-blink-features=MiddleClickAutoscroll"
    ];
  };
}
