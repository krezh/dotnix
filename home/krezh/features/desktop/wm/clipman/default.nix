{ pkgs, ... }:
{
  services.clipman = {
    enable = true;
    package = pkgs.clipman;
  };
}
