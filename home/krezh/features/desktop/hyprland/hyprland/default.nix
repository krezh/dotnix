{ pkgs, ... }:
{
  imports = [
    ./binds.nix
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
    settings = import ./settings.nix;
    plugins = [ ];
  };

  home.packages = with pkgs; [
    brightnessctl
    #nwg-displays
    grim
    slurp
    wl-screenrec
    hyprpicker
  ];
}
