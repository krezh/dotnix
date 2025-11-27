{ pkgs, ... }:
{
  home.packages = with pkgs; [
    # Screen/clipboard utilities
    brightnessctl
    grim
    wl-clipboard
    wl-screenrec

    # Hyprland-specific
    hyprpicker
    mission-center
    hyprmon
    hyprshade
    hyprdynamicmonitors
    bww
  ];
}
