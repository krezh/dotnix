{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.thor = {
    home-manager.users.${user} = {
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
        gaming
      ];
    };
  };
}
