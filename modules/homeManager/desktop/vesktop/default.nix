{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hmModules.desktop.vesktop;
  inherit (lib) mkEnableOption mkOption mkIf;
in
{
  options.hmModules.desktop.vesktop = {
    enable = mkEnableOption "Enable Vesktop";

    package = mkOption {
      type = pkgs.lib.types.package;
      default = pkgs.vesktop;
    };

    service = mkOption {
      type = lib.types.attrsOf lib.types.bool;
      default = {
        enable = true;
      };
    };

    settings = mkOption {
      type = lib.types.attrs;
      default = { };
    };

    vencord.settings = mkOption {
      type = lib.types.attrs;
      default = { };
    };
  };

  config = mkIf cfg.enable {
    programs.vesktop.enable = true;
    programs.vesktop.package = cfg.package;
    programs.vesktop.settings = cfg.settings;
    programs.vesktop.vencord.settings = cfg.vencord.settings;
    programs.vesktop.vencord.useSystem = false;
    services.arrpc.enable = true;

    systemd.user.services.vesktop = lib.mkIf cfg.service.enable {
      Install = {
        WantedBy = [ "graphical-session.target" ];
      };
      Unit = {
        Description = "A Custom Discord Client";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        RestartSec = 5;
      };
    };
  };
}
