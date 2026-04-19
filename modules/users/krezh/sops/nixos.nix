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

      system.activationScripts.sopsYubikey = {
        deps = [ "users" ];
        text = ''
          if [ ! -f ${config.sops.age.keyFile} ]; then
            echo "sopsYubikey: Generating Yubikey identity file..." >&2
            mkdir -p /home/${user}/.config/sops/age
            ${pkgs.age-plugin-yubikey}/bin/age-plugin-yubikey --identity > ${config.sops.age.keyFile}
            chown ${user}:users ${config.sops.age.keyFile}
            chmod 600 ${config.sops.age.keyFile}
            echo "sopsYubikey: Successfully created ${config.sops.age.keyFile}" >&2
          else
            echo "sopsYubikey: File already exists, skipping generation" >&2
          fi
        '';
      };

      sops = {
        age = {
          keyFile = "/home/${user}/.config/sops/age/keys.txt";
          sshKeyPaths = [ "/persist/etc/ssh/ssh_host_ed25519_key" ];
          plugins = [
            pkgs.age-plugin-fido2-hmac
            pkgs.age-plugin-yubikey
          ];
        };
        defaultSopsFile = ./secrets.sops.yaml;
        secrets = {
          "krezh/password" = {
            neededForUsers = true;
          };
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
