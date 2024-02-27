{ inputs, pkgs, ... }:
{
  imports = [
    inputs.hyprlock.homeManagerModules.hyprlock
  ];
  wayland.windowManager.hyprland = {
    enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd.enable = true;
    xwayland.enable = true;
    extraConfig = ''
      ${builtins.readFile ./hypr.conf}
      #plugin = ${inputs.hyprfocus.packages.${pkgs.system}.default}/lib/libhyprfocus.so
    '';
    settings = {
      exec-once = [
        "ags"
      ];
    };
  };

  programs.hyprlock = {
    enable = true;
  };
}
