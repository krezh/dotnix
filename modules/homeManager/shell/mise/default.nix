{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.hmModules.shell.mise;
  tomlFormat = pkgs.formats.toml { };
in
{
  options.hmModules.shell.mise = {
    enable = mkEnableOption "mise";

    package = mkPackageOption pkgs "mise" { };

    config = mkOption {
      type = tomlFormat.type;
      default = { };
    };

    enableFishIntegration = mkEnableOption "Fish Integration" // {
      default = true;
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    xdg.configFile."mise/settings.toml" = mkIf (cfg.config != { }) {
      source = tomlFormat.generate "mise-config" cfg.config;
    };
    programs = {
      fish.shellInit = mkIf cfg.enableFishIntegration (mkAfter ''
        ${cfg.package}/bin/mise hook-env | source
        ${cfg.package}/bin/mise activate fish | source
      '');
    };
  };
}
