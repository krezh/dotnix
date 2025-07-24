{
  config,
  lib,
  inputs,
  pkgs,
  ...
}:
{
  home = {
    sessionVariables = {
      DEFAULT_BROWSER = lib.getExe config.programs.vivaldi.package;
    };
  };

  programs = {
    vivaldi = {
      enable = true;
    };
  };

  home.packages = [ inputs.zen-browser.packages.${pkgs.system}.default ];
}
