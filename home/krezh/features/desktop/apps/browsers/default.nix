{ pkgs, ... }:
{
  home = {
    sessionVariables = {
      DEFAULT_BROWSER = "${pkgs.firefox}/bin/firefox";
    };
  };

  programs.firefox = {
    enable = true;
  };
}
