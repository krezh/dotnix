{
  pkgs,
  lib,
  config,
  ...
}:
let
  cfg = config.hmModules.shell.atuin;

  defaultConfig = {
    sync_address = cfg.sync_address;
    auto_sync = true;
    sync_frequency = "1m";
    search_mode = "fuzzy";
    store_failed = false;
    show_preview = true;
    filter_mode = "workspace";
    update_check = false;

    # Filter some commands we don't want to accidentally call from history
    history_filter = [
      "^(sudo reboot)$"
      "^(reboot)$"
    ];
  };
in
{
  options.hmModules.shell.atuin = {
    enable = lib.mkEnableOption "${lib.username} atuin";
    package = lib.mkPackageOption pkgs "atuin" { };

    sync_address = lib.mkOption {
      type = lib.types.str;
      default = "";
    };

    config = lib.mkOption {
      type = lib.types.attrs;
      default = { };
      description = "Additional atuin settings to merge with defaults";
    };
  };

  config = lib.mkIf cfg.enable {
    programs.atuin = {
      enable = true;
      package = cfg.package;

      enableBashIntegration = true;
      enableFishIntegration = true;
      enableZshIntegration = true;
      enableNushellIntegration = true;

      daemon = {
        enable = true;
      };

      flags = [ "--disable-up-arrow" ];

      settings = lib.mkMerge [
        defaultConfig
        cfg.config
      ];
    };
  };
}
