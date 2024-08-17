{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hmModules.desktop.vesktop;
  json = pkgs.formats.json { };
  inherit (lib) mkEnableOption mkOption mkIf;
in
{
  options.hmModules.desktop.vesktop = {
    enable = mkEnableOption "Enable Vesktop";

    package = mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.vesktop;
    };

    settings = mkOption {
      type = json.type;
      default = { };
    };
    extraConfig = mkOption {
      type = json.type;
      default = { };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    xdg.configFile."vesktop/settings.json" = mkIf (cfg.settings != { }) {
      source = json.generate "settings.json" cfg.settings;
    };
    xdg.configFile."vesktop/settings/settings.json" = mkIf (cfg.extraConfig != { }) {
      source = json.generate "settings.json" cfg.extraConfig;
    };
  };
}
