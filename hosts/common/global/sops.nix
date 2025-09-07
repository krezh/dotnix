{ config, ... }:

{
  imports = [ ];
  sops = {
    age = {
      keyFile = "/home/krezh/.config/sops/age/keys.txt";
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
      };
    };
  };
}
