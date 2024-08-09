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
      "wifi/Plexuz" = { };
      "wifi/Flyn" = { };
    };
    templates = {
      "nix_access_token.conf" = {
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
