{ inputs, ... }:
{
  flake.nixosConfigurations = inputs.self.lib.mkNixos {
    name = "thor-wsl";
    stateVersion = "24.05";
  };
}
