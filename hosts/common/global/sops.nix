{ config, ... }:

{
  imports = [ ];
  sops = {
    age = {
      keyFile = "/home/krezh/.config/sops/age/keys.txt";
    };
    # gnupg = {
    #   home = "/home/krezh/.gnupg";
    #   sshKeyPaths = [ ];
    # };
    defaultSopsFile = ../secrets.sops.yaml;

    secrets = {
      "github/token" = { };
    };
    templates = {
      "nix_access_token.conf" = {
        owner = "krezh";
        content = ''
          access-tokens = github.com=${config.sops.placeholder."github/token"}
        '';
      };
    };
  };
}
