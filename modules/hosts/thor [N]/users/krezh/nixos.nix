{ inputs, ... }:
{
  flake.modules.nixos.thor = {
    home-manager.users.krezh = {
      imports = with inputs.self.modules.homeManager; [
        system-desktop

        # Programs (module definitions)
        terminal
        editors
        browsers
        media
        launchers
        mail
        ai

        # Desktop environment
        hyprland
        niri
        desktop-shell
        desktop-utils

        # Gaming
        steam
      ];
    };
  };
}
