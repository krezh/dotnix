{ config, lib, ... }:

{
  fileSystems."/home".neededForBoot = true; # Make sure home is mounted before user services
  sops = {
    age = {
      keyFile = lib.mkDefault "/home/krezh/.config/sops/age/keys.txt";
      sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
    };
    defaultSopsFile = ../secrets.sops.yaml;

    secrets = {
      "github/token" = { };
      "smb/user" = { };
      "smb/pass" = { };
    };
    templates = {
      "nix_access_token.conf" = {
        owner = "krezh";
        content = ''
          access-tokens = github.com=${config.sops.placeholder."github/token"}
        '';
      };
      "jotunheim_homes_creds" = {
        owner = "krezh";
        content = ''
          username=${config.sops.placeholder."smb/user"}
          password=${config.sops.placeholder."smb/pass"}
        '';
        path = "/etc/nixos/smb-secrets";
      };
    };
  };
}
