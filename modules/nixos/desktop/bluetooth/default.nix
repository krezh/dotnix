{ config, lib, ... }:
let
  cfg = config.nixosModules.desktop.bluetooth;
in
{
  options.nixosModules.desktop.bluetooth = {
    enable = lib.mkEnableOption "bluetooth";
  };

  config = lib.mkIf cfg.enable {
    hardware.bluetooth.enable = true;
    services.blueman.enable = true;
  };
}
