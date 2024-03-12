{ pkgs, lib, config, ... }:
with lib;
let
  cfg = config.modules.shell.krew;
in {
  options.modules.shell.krew = {
    enable = mkEnableOption "krew";

    package = mkPackageOption pkgs "krew" { };

    enableFishIntegration = mkEnableOption "Fish Integration" // {
      default = true;
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    home.sessionPath = [
      PATH="${KREW_ROOT:-$HOME/.krew}/bin:$PATH"
    ];

    programs = {
      fish.shellInit = mkIf cfg.enableFishIntegration (mkAfter ''
        ${getExe cfg.package} completion fish | source
      '');
    };
  };
}
