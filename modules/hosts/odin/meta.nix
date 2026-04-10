{ inputs, ... }:
{
  flake.nixosConfigurations = inputs.self.lib.mkNixos {
    name = "odin";
    stateVersion = "24.05";
  };
}
