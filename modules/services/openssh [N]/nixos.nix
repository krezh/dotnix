{
  flake.modules.nixos.openssh = {
    config,
    lib,
    pkgs,
    ...
  }: {
    services.openssh = {
      enable = true;
      startWhenNeeded = true;
      openFirewall = true;
      settings = {
        UseDns = false;
        PasswordAuthentication = false;
        PermitRootLogin = lib.mkDefault "no";
        AllowGroups = ["sshusers"];
      };
      hostKeys = [
        {
          comment = "Hostkey for ${config.networking.hostName}";
          path = "/etc/ssh/ssh_host_ed25519_key";
          type = "ed25519";
        }
      ];
    };
    environment.systemPackages = with pkgs; [openssl];
  };
}
