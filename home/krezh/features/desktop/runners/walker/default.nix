{
  pkgs,
  inputs,
  ...
}:
{
  imports = [
    inputs.walker.homeManagerModules.default
  ];

  programs.walker = {
    enable = true;
    package = inputs.walker.packages.${pkgs.system}.default;
    runAsService = true;

    config = {
      #theme_base = [ "catppuccin" ];

      activation_mode.disabled = true;
      ignore_mouse = false;
    };
  };
}
