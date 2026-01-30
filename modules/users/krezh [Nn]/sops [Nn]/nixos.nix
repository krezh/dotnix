let
  user = "krezh";
in
{
  flake.modules.nixos.${user} =
    { config, ... }:
    {
      # NixOS-level sops secrets
      sops = {
        age = {
          keyFile = "/home/${user}/.config/sops/age/keys.txt";
          sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
        };
        defaultSopsFile = ./secrets.sops.yaml;
        secrets = {
          "github/token" = { };
          "smb/user" = { };
          "smb/pass" = { };
        };
        templates = {
          "nix_access_token.conf" = {
            owner = user;
            content = ''
              access-tokens = github.com=${config.sops.placeholder."github/token"}
            '';
          };
          "jotunheim_homes_creds" = {
            owner = user;
            content = ''
              username=${config.sops.placeholder."smb/user"}
              password=${config.sops.placeholder."smb/pass"}
            '';
            path = "/etc/nixos/smb-secrets";
          };
        };
      };
    };
}
