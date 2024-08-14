{ config, lib, ... }:
let
  cfg = config.nixosModules.desktop.battery;
in
{
  options.nixosModules.desktop.battery = {
    enable = lib.mkEnableOption "battery";
  };

  config = lib.mkIf cfg.enable {

    services = {
      upower.enable = true;
      power-profiles-daemon.enable = true;
    };
  };
}
