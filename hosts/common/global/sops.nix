{ inputs, ... }:

{
  imports = [
    inputs.sops-nix.nixosModules.sops
  ];
  sops = {
    age.keyFile = "/home/krezh/.config/sops/age/keys.txt";
    defaultSopsFile = ../secrets.sops.yaml;
    secrets = {
      "krezh-password" = {
        neededForUsers = true;
      };
    };
  };
}
