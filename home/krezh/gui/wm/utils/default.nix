{ pkgs, ... }:
{
  home.packages = with pkgs; [
    # Screen/clipboard utilities
    brightnessctl
    grim
    wl-clipboard
    wl-screenrec
    recshot
    gulp

    # Hyprland-specific
    hyprpicker
    mission-center
    hyprmon
    hyprshade
    hyprdynamicmonitors
    bww
  ];
}
