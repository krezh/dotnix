{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.modules.desktop.wlogout;
in
{
  options.modules.desktop.wlogout = {
    enable = mkEnableOption "wlogout";

    package = mkPackageOption pkgs "wlogout" { };

    style = mkOption {
      type = path.type;
      default = { };
    };

    layout = mkOption {
      type = path.type;
      default = { };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    xdg.configFile."wlogout/style.css" = mkIf (cfg.style != { }) { source = cfg.style; };
    xdg.configFile."wlogout/layout" = mkIf (cfg.layout != { }) { source = cfg.layout; };
  };
}
