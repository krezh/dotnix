{ inputs, pkgs, ... }:
let
  hyprpanelFlake = inputs.hyprpanel.packages.${pkgs.system}.default;
in
{
  hmModules.desktop.hyprpanel = {
    enable = true;
    package = hyprpanelFlake;
  };

  home.packages = [ hyprpanelFlake ];
}
