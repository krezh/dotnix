{ pkgs, config, ... }:
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
    (writeShellScriptBin "hyprexit" ''
      ${hyprland}/bin/hyprctl dispatch exit
      ${systemd}/bin/loginctl terminate-user ${config.home.username}
    '')
  ];
}
