{ config, lib, ... }:
let
  cfg = config.nixosModules.desktop.openssh;
in
{
  options.nixosModules.desktop.openssh = {
    enable = lib.mkEnableOption "openssh";
  };

  config = lib.mkIf cfg.enable {
    services.openssh = {
      enable = true;
      startWhenNeeded = true;
      settings = {
        UseDns = true;
        PasswordAuthentication = false;
        PermitRootLogin = "no";
        AllowGroups = [ "sshusers" ];
      };
      hostKeys = [
        {
          comment = "Hostkey for ${config.networking.hostName}";
          path = "/etc/ssh/ssh_host_ed25519_key";
          type = "ed25519";
        }
      ];
    };
  };
}
