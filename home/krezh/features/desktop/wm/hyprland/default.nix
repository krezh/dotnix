{ inputs, pkgs, ... }: {
  imports = [ inputs.hyprlock.homeManagerModules.hyprlock ];
  wayland.windowManager.hyprland = {
    enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd.enable = true;
    xwayland.enable = true;
    extraConfig = ''
      ${builtins.readFile ./hypr.conf}
      bind = $mainMod,L,exec,${pkgs.hyprlock}/bin/hyprlock
      bind = $mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout
      bind = $mainMod, K, exec, $term ${pkgs.hyprkeys}/bin/hyprkeys -o markdown -c $XDG_CONFIG_HOME/hypr/hypr.conf | ${pkgs.glow}/bin/glow -s dark

    '';
    settings = { };
  };

  programs.hyprlock = { enable = true; };

  home.packages = with pkgs; [
    inputs.hyprkeys.packages.${pkgs.system}.hyprkeys
    wlogout
    glow
  ];

}
