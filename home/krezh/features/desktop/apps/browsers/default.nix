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
  home.packages = with pkgs; [ inputs.browser-previews.packages.${pkgs.system}.google-chrome ];
}
