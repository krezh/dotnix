{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.nixosModules.desktop.openrgb;
in
{
  options.nixosModules.desktop.openrgb = {
    enable = lib.mkEnableOption "openrgb";
  };

  config = lib.mkIf cfg.enable {

    services.hardware.openrgb.enable = true;
    environment.systemPackages = with pkgs; [
      openrgb-with-all-plugins
    ];
  };
}
