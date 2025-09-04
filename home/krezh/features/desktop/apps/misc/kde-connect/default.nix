{ pkgs, ... }:
{
  services.kdeconnect.enable = true;
  services.kdeconnect.indicator = true;
  services.kdeconnect.package = pkgs.kdePackages.kdeconnect-kde;
}
