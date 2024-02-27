{ inputs, pkgs, ... }:
{
  programs.ags = {
    enable = true;
    package = inputs.ags.packages.${pkgs.system}.default;
    configDir = ./config;
    extraPackages = [ pkgs.libsoup_3 ];
  };
}
