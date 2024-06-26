{ inputs, pkgs, ... }:
{
  imports = [ ];
  wayland.windowManager.hyprland = {
    enable = true;
    systemd.enable = true;
    xwayland.enable = false;
    extraConfig = ''
      ${builtins.readFile ./hypr.conf}

      # Generated from Nix
      bind = $mainMod,L,exec,${pkgs.hyprlock}/bin/hyprlock
      bind = $mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout
      bind = $mainMod,R,exec,${inputs.anyrun.packages.${pkgs.system}.default}/bin/anyrun --plugins ${
        inputs.anyrun.packages.${pkgs.system}.applications
      }/lib/libapplications.so
      bind = $mainMod,K,exec,${pkgs.hyprkeys}/bin/hyprkeys -b -r | anyrun --plugins ${
        inputs.anyrun.packages.${pkgs.system}.stdin
      }/lib/libstdin.so
      # Generated from Nix
    '';
    settings = { };
    plugins = [
      # inputs.hyprfocus.packages.${pkgs.system}.hyprfocus
      # inputs.hyprgrass.packages.${pkgs.system}.default
      # inputs.hyprland-plugins.packages.${pkgs.system}.hyprexpo
    ];
  };

  programs.hyprlock = {
    enable = true;
    package = inputs.hyprlock.packages.${pkgs.system}.hyprlock;
  };

  home.packages = with pkgs; [
    inputs.hyprkeys.packages.${pkgs.system}.hyprkeys
    inputs.xdg-portal-hyprland.packages.${pkgs.system}.default
    wlogout
    glow
  ];
}
