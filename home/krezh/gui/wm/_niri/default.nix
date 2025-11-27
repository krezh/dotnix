{ pkgs, ... }:
{
  imports = [
    ./input.nix
    ./outputs.nix
    ./layout.nix
    ./animations.nix
    ./window-rules.nix
    ./binds.nix
    ./startup.nix
    ./workspaces.nix
    ./general.nix
  ];

  programs.niri.settings = {
    xwayland-satellite = {
      enable = true;
      path = "${pkgs.xwayland-satellite}/bin/xwayland-satellite";
    };
  };

  home.packages = with pkgs; [
    brightnessctl
    grim
    slurp
    wl-clipboard
    wl-screenrec
    swaylock
    swayidle
    mission-center
    xwayland-satellite
  ];

  home.sessionVariables = {
    NIXOS_OZONE_WL = "1";
    MOZ_ENABLE_WAYLAND = "1";
    QT_QPA_PLATFORM = "wayland";
    QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
    SDL_VIDEODRIVER = "wayland";
    _JAVA_AWT_WM_NONREPARENTING = "1";
  };
}
