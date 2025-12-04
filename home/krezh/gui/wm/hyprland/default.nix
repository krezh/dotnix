{ pkgs, ... }:
{
  imports = [
    ./settings.nix
    ./binds.nix
    ./monitors.nix
    ./smartgaps.nix
    ./rules.nix
    # ./scrolling.nix
    ./animations.nix
  ];

  services.polkit-gnome.enable = true;

  wayland.windowManager.hyprland = {
    enable = true;
    xwayland.enable = true;
    systemd = {
      enable = true;
      enableXdgAutostart = true;
      variables = [ "--all" ];
    };
    plugins = with pkgs.hyprlandPlugins; [
      hyprexpo
    ];
  };
}
