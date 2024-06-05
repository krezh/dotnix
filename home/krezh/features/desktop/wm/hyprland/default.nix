{ inputs, pkgs, config, ... }: {
  imports = [ ];
  wayland.windowManager.hyprland = {
    enable = true;
    # package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd.enable = true;
    xwayland.enable = true;
    extraConfig = ''
      ${builtins.readFile ./hypr.conf}

      # Generated from Nix
      bind = $mainMod,L,exec,${pkgs.hyprlock}/bin/hyprlock
      bind = $mainMod,ESCAPE,exec,${pkgs.wlogout}/bin/wlogout
      #bind = $mainMod,K,exec,$term start -- ${pkgs.hyprkeys}/bin/hyprkeys -b -m -c ${config.xdg.configHome}/hypr/hyprland.conf | ${pkgs.glow}/bin/glow -p
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
