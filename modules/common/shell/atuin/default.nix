{
  pkgs,
  lib,
  config,
  ...
}:
let
  cfg = config.modules.shell.atuin;
  defaultConfig = import ./defaultConfig.nix { sync_address = cfg.sync_address; };
in
{
  options.modules.shell.atuin = {
    enable = lib.mkEnableOption "${lib.username} atuin";
    package = lib.mkPackageOption pkgs "atuin" { };

    sync_address = lib.mkOption {
      type = lib.types.str;
      default = "";
    };
    config = lib.mkOption {
      type = lib.types.attrs;
      default = { };
    };
  };

  config = lib.mkIf cfg.enable {
    programs.atuin = {
      enable = true;
      package = cfg.package;

      flags = [ "--disable-up-arrow" ];

      settings = lib.mkMerge [
        defaultConfig
        cfg.config
      ];
    };
  };
}
