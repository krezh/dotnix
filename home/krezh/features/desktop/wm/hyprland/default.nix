{ inputs, pkgs, ... }:
{
  imports = [
    inputs.hyprland.homeManagerModules.default
    ./binds.nix
  ];

  wayland.windowManager.hyprland = {
    enable = true;
    catppuccin.enable = true;
    xwayland.enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd = {
      enable = true;
      variables = [ "--all" ]; # https://wiki.hyprland.org/Nix/Hyprland-on-Home-Manager/#programs-dont-work-in-systemd-services-but-do-on-the-terminal
    };

    extraConfig = '''';

    settings = import ./settings.nix;

    plugins = [
      inputs.hyprfocus.packages.${pkgs.system}.hyprfocus
      # inputs.hyprhook.packages.${pkgs.system}.hyprhook
      # inputs.hyprland-plugins.packages.${pkgs.system}.hyprexpo
    ];
  };

  home.packages = with pkgs; [
    clipman
    flameshot
    brightnessctl
    light
  ];
}
