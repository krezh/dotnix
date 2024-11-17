{
  config,
  pkgs,
  lib,
  ...
}:
with { inherit (lib) mkOption mkEnableOption types; };
let
  cfg = config.hmModules.desktop.shikane;
  toml = pkgs.formats.toml { };
in
{
  options.hmModules.desktop.shikane = {
    enable = lib.mkEnableOption "shikane";

    systemdTarget = mkOption {
      type = types.str;
      default = "hyprland-session.target";
      description = ''
        Systemd target to bind to.
      '';
    };
    package = mkOption {
      type = types.package;
      default = pkgs.shikane;
      description = ''
        shikane derivation to use.
      '';
    };
    config = mkOption {
      type = types.attrs;
      default = { };
      description = ''
        Configuration for shikane.
      '';
    };
  };

  config = lib.mkIf cfg.enable {

    xdg.configFile."shikane/config.toml" = {
      source = toml.generate "shikane/config.toml" cfg.config;
    };

    systemd.user.services.shikane = {
      Unit = {
        Description = "Dynamic output configuration";
        Documentation = "man:shikane(1)";
        PartOf = cfg.systemdTarget;
        Requires = cfg.systemdTarget;
        After = cfg.systemdTarget;
      };

      Service = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/shikane -c ${
          config.home.homeDirectory + "/" + config.xdg.configFile."shikane/config.toml".target
        }";
        Restart = "always";
      };

      Install = {
        WantedBy = [ cfg.systemdTarget ];
      };
    };

    home.packages = [
      pkgs.shikane # For the CLI (saving profiles)
    ];
  };
}
