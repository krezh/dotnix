{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
let
  cfg = config.hmModules.shell.k9s;
  yamlFormat = pkgs.formats.yaml { };
in
{
  options.hmModules.shell.k9s = {
    enable = mkEnableOption "k9s";

    package = mkPackageOption pkgs "k9s" { };

    config = mkOption {
      type = yamlFormat.type;
      default = { };
    };

    aliases = mkOption {
      type = yamlFormat.type;
      default = { };
    };

    plugins = mkOption {
      type = yamlFormat.type;
      default = { };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    xdg.configFile."k9s/config.yaml" = mkIf (cfg.config != { }) {
      source = yamlFormat.generate "k9s-config" cfg.config;
    };
    xdg.configFile."k9s/aliases.yaml" = mkIf (cfg.aliases != { }) {
      source = yamlFormat.generate "k9s-aliases" cfg.aliases;
    };
    xdg.configFile."k9s/plugins.yaml" = mkIf (cfg.plugins != { }) {
      source = yamlFormat.generate "k9s-plugins" cfg.plugins;
    };
    xdg.configFile."k9s/skins" = mkIf (hasPrefix "catppuccin" cfg.config.k9s.ui.skin) {
      source =
        pkgs.fetchFromGitHub {
          owner = "catppuccin";
          repo = "k9s";
          rev = "fdbec82284744a1fc2eb3e2d24cb92ef87ffb8b4";
          sha256 = "sha256-9h+jyEO4w0OnzeEKQXJbg9dvvWGZYQAO4MbgDn6QRzM=";
        }
        + "/dist";
    };
  };
}
