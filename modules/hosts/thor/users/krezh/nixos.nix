{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.thor = {
    imports = [
      inputs.self.modules.nixos.${user}
      inputs.self.modules.nixos.protonvpn
    ];
    security.pam.services.${user}.enableGnomeKeyring = true;
    users.users.${user}.extraGroups = [ "tss" ]; # tss group has access to TPM devices

    home-manager.users.${user} = {
      targets.network-online.enable = true;
      autostart.enableXdgAutostart = true;

      imports = with inputs.self.modules.homeManager; [
        system-desktop
        desktop-shell
        desktop-utils
        terminal
        editors
        browsers
        media
        launchers
        mail
        ai
        kubernetes
        office
        hyprland
        gaming
      ];
    };
  };
}
