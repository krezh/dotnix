{ inputs, lib, ... }:
{
  options.flake.lib = lib.mkOption {
    type = lib.types.attrsOf lib.types.unspecified;
    default = { };
  };

  config.flake.lib = {
    mkNixos =
      {
        name,
        system ? "x86_64-linux",
        stateVersion ? null,
      }:
      {
        ${name} = inputs.nixpkgs.lib.nixosSystem {
          specialArgs = {
            inherit lib;
          };
          modules = [
            inputs.self.modules.nixos.${name}
            {
              nixpkgs.hostPlatform = lib.mkDefault system;
              networking.hostName = lib.mkDefault name;
            }
          ]
          ++ lib.optional (stateVersion != null) { system.stateVersion = stateVersion; };
        };
      };
  };
}
