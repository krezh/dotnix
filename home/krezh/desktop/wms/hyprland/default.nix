{ pkgs, ... }:
{
  imports = [
    ./settings.nix
    ./binds.nix
    ./monitors.nix
    ./smartgaps.nix
    ./rules.nix
    #./scrolling.nix
  ];

  wayland.windowManager.hyprland = {
    enable = true;
    xwayland.enable = true;
    systemd = {
      enable = true;
      enableXdgAutostart = false;
      variables = [
        "--all"
      ]; # https://wiki.hyprland.org/Nix/Hyprland-on-Home-Manager/#programs-dont-work-in-systemd-services-but-do-on-the-terminal
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
    hyprshade # For display color/gamma adjustments
    hyprdynamicmonitors
  ];
}
