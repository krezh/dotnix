{ inputs, ... }:
{
  flake.nixosConfigurations = inputs.self.lib.mkNixos {
    name = "phone";
    system = "aarch64-linux";
    stateVersion = "24.05";
  };
}
