{
  flake.modules.nixos.openssh =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    {
      services.openssh = {
        enable = true;
        startWhenNeeded = true;
        openFirewall = true;
        settings = {
          UseDns = false;
          PasswordAuthentication = false;
          PermitRootLogin = lib.mkDefault "no";
          AllowGroups = [ "sshusers" ];
          KexAlgorithms = [
            "mlkem768x25519-sha256"
            "sntrup761x25519-sha512@openssh.com"
            "curve25519-sha256"
            "curve25519-sha256@libssh.org"
            "diffie-hellman-group16-sha512"
            "diffie-hellman-group18-sha512"
            "ecdh-sha2-nistp256"
            "ecdh-sha2-nistp384"
            "ecdh-sha2-nistp521"
          ];
        };
        hostKeys = [
          {
            comment = "Hostkey for ${config.networking.hostName}";
            path = "/etc/ssh/ssh_host_ed25519_key";
            type = "ed25519";
          }
        ];
      };
      environment.systemPackages = with pkgs; [ openssl ];
    };
}
