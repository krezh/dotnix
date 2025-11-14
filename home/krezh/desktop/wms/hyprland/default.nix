{ pkgs, ... }:
{
  imports = [
    ./settings.nix
    ./binds.nix
    ./monitors.nix
    ./smartgaps.nix
    ./rules.nix
    # ./scrolling.nix
  ];

  services.polkit-gnome.enable = true;

  wayland.windowManager.hyprland = {
    enable = true;
    xwayland.enable = true;
    systemd = {
      enable = true;
      enableXdgAutostart = false;
      variables = [ "--all" ];
    };
    plugins = with pkgs.hyprlandPlugins; [
      hyprexpo
    ];
  };

  home.packages = with pkgs; [
    brightnessctl
    grim
    slurp
    wl-clipboard
    wl-screenrec
    hyprpicker
    mission-center
    hyprmon
    hyprshade
    hyprdynamicmonitors
    bww
  ];
}
