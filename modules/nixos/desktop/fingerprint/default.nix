{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.nixosModules.desktop.fingerprint;
in
{
  options.nixosModules.desktop.fingerprint = {
    enable = lib.mkEnableOption "fingerprint";
  };

  config = lib.mkIf cfg.enable {

    environment.systemPackages = [ pkgs.fprintd ];

    services = {
      fprintd.enable = true;
    };
  };
}
