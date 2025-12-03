{ config, lib, ... }:
let
  cfg = config.nixosModules.battery;
in
{
  options.nixosModules.battery = {
    enable = lib.mkEnableOption "battery";
  };

  config = lib.mkIf cfg.enable {

    services = {
      upower.enable = true;
      power-profiles-daemon.enable = true;
    };
  };
}
