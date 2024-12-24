{
  pkgs,
  inputs,
  config,
  lib,
  ...
}:
{
  home = {
    sessionVariables = {
      DEFAULT_BROWSER = lib.getExe config.programs.vivaldi.package;
    };
  };

  programs = {
    firefox = {
      enable = true;
    };
  };

  programs = {
    vivaldi = {
      enable = true;
    };
  };

  home.packages = [ inputs.browser-previews.packages.${pkgs.system}.google-chrome ];
}
