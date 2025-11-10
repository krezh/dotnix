{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.hmModules.shell.krew;
in
{
  options.hmModules.shell.krew = {
    enable = mkEnableOption "krew";

    package = mkPackageOption pkgs "krew" { };

    enableFishIntegration = mkEnableOption "Fish Integration" // {
      default = true;
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    home.sessionVariables = {
      KREW_ROOT = "$HOME/.krew";
    };
    home.sessionPath = [ "$KREW_ROOT/bin" ];
  };
}
