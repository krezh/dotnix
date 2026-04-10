{ inputs, ... }:
{
  flake.nixosConfigurations = inputs.self.lib.mkNixos {
    name = "steamdeck";
    stateVersion = "24.05";
  };
}
