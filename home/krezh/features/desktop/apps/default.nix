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
  ];
  home.packages = with pkgs; [
    resources
  ];
}
