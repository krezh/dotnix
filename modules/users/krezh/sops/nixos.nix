let
  user = "krezh";
in
{
  flake.modules.nixos.${user} =
    { config, pkgs, ... }:
    {
      environment = {
        systemPackages = with pkgs; [
          age-plugin-yubikey
          age-plugin-fido2-hmac
        ];
      };
      sops = {
        age = {
          keyFile = "/home/${user}/.config/sops/age/keys.txt";
          sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
          plugins = [
            pkgs.age-plugin-fido2-hmac
            pkgs.age-plugin-yubikey
          ];
        };
        defaultSopsFile = ./secrets.sops.yaml;
        secrets = {
          "github/token" = { };
          "smb/user" = { };
          "smb/pass" = { };
          "nixbuild/key" = { };
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
