{ inputs, ... }:
{
  flake.nixosConfigurations = inputs.self.lib.mkNixos {
    name = "jotunheim";
    stateVersion = "24.05";
  };
}
