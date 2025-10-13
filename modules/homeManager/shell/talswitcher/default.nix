{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.hmModules.shell.talswitcher;
in
{
  options.hmModules.shell.talswitcher = {
    enable = mkEnableOption "talswitcher";
    package = mkPackageOption pkgs "talswitcher" { };
    enableFishIntegration = mkEnableOption "Fish Integration" // {
      default = true;
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    home.sessionVariables = {
      TALOSCONFIG_DIR = lib.mkDefault "${config.home.homeDirectory}/.talos/configs";
    };
    programs = {
      fish.shellInit = mkIf cfg.enableFishIntegration (mkAfter ''
        alias ts '${lib.getExe cfg.package}'
        alias tsc '${lib.getExe cfg.package} ctx'
      '');
    };
  };
}
