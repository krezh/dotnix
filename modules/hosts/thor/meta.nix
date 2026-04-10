{ inputs, ... }:
{
  flake.nixosConfigurations = inputs.self.lib.mkNixos {
    name = "thor";
    stateVersion = "24.05";
  };
}
