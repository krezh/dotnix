{ inputs, ... }:
{
  flake.nixosConfigurations = inputs.self.lib.mkNixos {
    name = "jotunheim";
    system = "x86_64-linux";
    stateVersion = "24.05";
  };
}
