{
  config,
  lib,
  ...
}:
let
  cfg = config.nixosModules.desktop.netbird;
in
{
  options.nixosModules.desktop.netbird = {
    enable = lib.mkEnableOption "netbird";
  };

  config = lib.mkIf cfg.enable {
    services.netbird = {
      enable = true;
      ui.enable = true;
      useRoutingFeatures = "both";
    };
  };
}
