{
  config,
  lib,
  pkgs,
  ...
}:
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
      openFirewall = true;
      settings = {
        UseDns = false;
        PasswordAuthentication = false;
        PermitRootLogin = lib.mkDefault "no";
        AllowGroups = [ "sshusers" ];
      };
      hostKeys = [
        {
          comment = "Hostkey for ${config.networking.hostName}";
          path = "/etc/ssh/ssh_host_ed25519_key";
          type = "ed25519";
        }
      ];
      #authorizedKeysFiles = [ "${inputs.ssh-keys.outPath}" ];
    };
    environment.systemPackages = with pkgs; [
      openssl
    ];
  };
}
