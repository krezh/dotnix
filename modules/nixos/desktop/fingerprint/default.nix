{ config, lib, ... }:
let
  cfg = config.nixosModules.desktop.fingerprint;
in
{
  options.nixosModules.desktop.fingerprint = {
    enable = lib.mkEnableOption "fingerprint";
  };

  config = lib.mkIf cfg.enable {

    services = {
      fprintd.enable = true;
      # fprintd.tod.enable = true;
      # fprintd.tod.driver = pkgs.libfprint-2-tod1-vfs0090;
      # fprintd.tod.driver = pkgs.libfprint-2-tod1-goodix;
    };
  };
}
