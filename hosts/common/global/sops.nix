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
      "passwords/krezh" = {
        neededForUsers = true;
      };
      "passwords/dummy" = {
        neededForUsers = true;
      };
      "github/token" = { };
      "wifi/Plexuz" = { };
      "wifi/Flyn" = { };
    };
    templates = {
      "nix_access_token.conf" = {
        owner = "krezh";
        content = ''
          access-tokens = github.com=${config.sops.placeholder."github/token"}
        '';
      };
      "networkManager.env" = {
        content = ''
          Plexuz=${config.sops.placeholder."wifi/Plexuz"}
          Flyn=${config.sops.placeholder."wifi/Flyn"}
        '';
      };
    };
  };
}
