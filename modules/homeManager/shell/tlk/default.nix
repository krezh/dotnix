{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.hmModules.shell.tlk;
  jsonFormat = pkgs.formats.json { };
in
{
  options.hmModules.shell.tlk = {
    enable = mkEnableOption "tlk";

    package = mkPackageOption pkgs "tlk" { };

    config = mkOption {
      type = jsonFormat.type;
      default = { };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    xdg.configFile."tlk/config.json" = mkIf (cfg.config != { }) {
      source = jsonFormat.generate "tlk-config" cfg.config;
    };
  };
}
