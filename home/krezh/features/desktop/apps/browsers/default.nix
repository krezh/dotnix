{ pkgs, inputs, ... }:
{
  home = {
    sessionVariables = {
      DEFAULT_BROWSER = "${pkgs.firefox}/bin/firefox";
    };
  };

  programs = {
    firefox = {
      enable = true;
    };
  };
  home.packages = with pkgs; [ inputs.browser-previews.packages.${pkgs.system}.google-chrome ];
}
