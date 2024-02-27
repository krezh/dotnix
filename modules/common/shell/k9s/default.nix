{ pkgs, lib, config, ... }:
with lib;
let
  cfg = config.modules.shell.k9s;
  yamlFormat = pkgs.formats.yaml { };
in
{
  options.modules.shell.k9s = {
    enable = mkEnableOption "k9s";

    package = mkPackageOption pkgs "" { };

    config = mkOption {
      type = yamlFormat.type;
      default = { };
    };

    aliases = mkOption {
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
    xdg.configFile."k9s/skins" = mkIf (hasPrefix "catppuccin" cfg.config.k9s.ui.skin) {
      source = pkgs.fetchFromGitHub
        {
          owner = "catppuccin";
          repo = "k9s";
          rev = "590a762110ad4b6ceff274265f2fe174c576ce96";
          sha256 = "sha256-EBDciL3F6xVFXvND+5duT+OiVDWKkFMWbOOSruQ0lus=";
        } + "/dist";
    };
  };
}
