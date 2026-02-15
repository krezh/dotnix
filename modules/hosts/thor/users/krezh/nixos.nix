{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.thor = {
    imports = with inputs.self.modules.homeManager; [
      inputs.self.modules.nixos.${user}
    ];
    security.pam.services.${user}.enableGnomeKeyring = true;

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
        desktop-shell
        desktop-utils

        # Gaming
        gaming
      ];
    };
  };
}
