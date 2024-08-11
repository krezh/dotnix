{ inputs, pkgs, ... }:
let
  hyprpanelFlake = inputs.hyprpanel.packages.${pkgs.system}.default;
in
{
  modules.desktop.hyprpanel = {
    enable = true;
    package = hyprpanelFlake;
  };

  home.packages = with pkgs; [ hyprpanelFlake ];
}
