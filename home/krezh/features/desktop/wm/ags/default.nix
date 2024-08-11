{ inputs, pkgs, ... }:
{
  home.packages = with pkgs; [ inputs.hyprpanel.packages.${pkgs.system}.default ];
}
