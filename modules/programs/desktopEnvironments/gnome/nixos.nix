{ ... }:
{
  flake.modules.nixos.gnome =
    { pkgs, ... }:
    {
      services.desktopManager.gnome.enable = true;
      services.gnome.core-apps.enable = true;
      services.gnome.core-developer-tools.enable = false;
      services.gnome.games.enable = false;

      programs.dconf.enable = true;
      programs.dconf.profiles.user.databases = [
        {
          settings = {
            "org/gnome/mutter" = {
              experimental-features = [
                "scale-monitor-framebuffer" # Enables fractional scaling (125% 150% 175%)
                "variable-refresh-rate" # Enables Variable Refresh Rate (VRR) on compatible displays
                "xwayland-native-scaling" # Scales Xwayland applications to look crisp on HiDPI screens
                "autoclose-xwayland" # automatically terminates Xwayland if all relevant X11 clients are gone
              ];
            };
          };
        }
      ];

      environment.gnome.excludePackages = with pkgs; [
        gnome-tour
        gnome-user-docs
        epiphany
        geary
        gnome-music
        gnome-photos
        gnome-maps
        gnome-contacts
        gnome-weather
        gnome-connections
      ];
    };
}
