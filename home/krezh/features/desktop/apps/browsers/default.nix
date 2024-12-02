{
  pkgs,
  inputs,
  config,
  ...
}:
{
  home = {
    sessionVariables = {
      DEFAULT_BROWSER = "${config.programs.vivaldi.package}/bin/vivaldi";
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
