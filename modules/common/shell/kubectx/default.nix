{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.modules.shell.kubectx;
in
{
  options.modules.shell.kubectx = {
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
