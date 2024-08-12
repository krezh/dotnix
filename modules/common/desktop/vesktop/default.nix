{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.modules.desktop.vesktop;
  json = pkgs.formats.json { };
  inherit (lib) mkEnableOption mkOption mkIf;
in
{
  options.modules.desktop.vesktop = {
    enable = mkEnableOption "Enable Vesktop";

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
    home.packages = with pkgs; [ vesktop ];
    xdg.configFile."vesktop/settings.json" = mkIf (cfg.settings != { }) {
      source = json.generate "settings.json" cfg.settings;
    };
    xdg.configFile."vesktop/settings/settings.json" = mkIf (cfg.extraConfig != { }) {
      source = json.generate "settings.json" cfg.extraConfig;
    };
  };
}
