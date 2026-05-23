{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.services.gotify-desktop;
      tomlFormat = pkgs.formats.toml { };

      settings = {
        gotify = {
          inherit (cfg) url;
          token = {
            command = "cat ${cfg.tokenFile}";
          };
          auto_delete = cfg.autoDelete;
        };
        notification.min_priority = cfg.minPriority;
      }
      // lib.optionalAttrs (cfg.onMsgCommand != null) {
        action.on_msg_command = cfg.onMsgCommand;
      };
    in
    {
      options.services.gotify-desktop = {
        enable = lib.mkEnableOption "gotify-desktop notification daemon";

        url = lib.mkOption {
          type = lib.types.str;
          description = "Gotify server URL";
          example = "https://gotify.example.com";
        };

        tokenFile = lib.mkOption {
          type = lib.types.path;
          description = "Path to a file containing the Gotify auth token";
          example = "/run/secrets/gotify-token";
        };

        autoDelete = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Delete messages from the server after displaying them";
        };

        minPriority = lib.mkOption {
          type = lib.types.int;
          default = 0;
          description = "Minimum message priority to display";
        };

        onMsgCommand = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Shell command to run on each message. GOTIFY_MSG_PRIORITY, GOTIFY_MSG_TITLE, and GOTIFY_MSG_TEXT are set in the environment";
          example = "notify-send \"$GOTIFY_MSG_TITLE\" \"$GOTIFY_MSG_TEXT\"";
        };
      };

      config = lib.mkIf cfg.enable {
        xdg.configFile."gotify-desktop/config.toml".source =
          tomlFormat.generate "gotify-desktop-config.toml" settings;

        systemd.user.services.gotify-desktop = {
          Unit = {
            Description = "Gotify desktop notification daemon";
            After = [
              "graphical-session.target"
              "network-online.target"
            ];
            Wants = [ "network-online.target" ];
            PartOf = [ "graphical-session.target" ];
          };
          Service = {
            ExecStart = lib.getExe pkgs.gotify-desktop;
            Restart = "on-failure";
            RestartSec = 5;
          };
          Install.WantedBy = [ "graphical-session.target" ];
        };
      };
    };
}
