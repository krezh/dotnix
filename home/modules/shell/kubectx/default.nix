{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.hmModules.shell.kubectx;
in
{
  options.hmModules.shell.kubectx = {
    enable = mkEnableOption "kubectx";

    package = mkPackageOption pkgs "kubectx" { };

    enableFishIntegration = mkEnableOption "Fish Integration" // {
      default = true;
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    programs = {
      fish.shellInit = mkIf cfg.enableFishIntegration (mkAfter ''
        alias ktx '${cfg.package}/bin/kubectx'
        alias kns '${cfg.package}/bin/kubens'
      '');
    };
  };
}
