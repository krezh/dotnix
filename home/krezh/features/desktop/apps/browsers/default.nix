{ pkgs, inputs, ... }:
{
  home = {
    sessionVariables = {
      DEFAULT_BROWSER = "${
        inputs.browser-previews.packages.${pkgs.system}.google-chrome
      }/bin/google-chrome-stable";
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
