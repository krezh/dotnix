{ inputs, config, ... }:

{
  imports = [ inputs.sops-nix.nixosModules.sops ];
  sops = {
    age = {
      keyFile = "/home/krezh/.config/sops/age/keys.txt";
    };
    defaultSopsFile = ../secrets.sops.yaml;
    secrets = {
      "krezh-password" = {
        neededForUsers = true;
      };
      "github/token" = { };
    };
    templates."nix_access_token.conf" = {
      content = ''
        access-tokens = github.com=${config.sops.placeholder."github/token"}
      '';
    };
  };
}
