{ pkgs, ... }:
{
  imports = [
    ./terminal
    ./editors
    ./browsers
    ./chat
    ./notes
    ./mail
    ./media
    ./misc
  ];
  home.packages = with pkgs; [
    resources
    seabird
  ];
}
