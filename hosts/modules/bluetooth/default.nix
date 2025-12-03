{ config, lib, ... }:
let
  cfg = config.nixosModules.bluetooth;
in
{
  options.nixosModules.bluetooth = {
    enable = lib.mkEnableOption "bluetooth";
  };

  config = lib.mkIf cfg.enable {
    hardware.bluetooth.enable = true;
  };
}
