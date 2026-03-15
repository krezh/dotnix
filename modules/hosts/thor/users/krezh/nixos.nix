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
    users.users.${user}.extraGroups = [ "tss" ]; # tss group has access to TPM devices

    home-manager.users.${user} = {
      imports = with inputs.self.modules.homeManager; [
        system-desktop

        terminal
        editors
        browsers
        media
        launchers
        mail
        ai
        kubernetes

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
