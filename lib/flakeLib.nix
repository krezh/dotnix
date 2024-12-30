{
  inputs,
  outputs,
  lib,
  ...
}:
let
  # function to make `pkgs` for defined system with my overlays
  mkPkgsWithSystem =
    system:
    import inputs.nixpkgs {
      inherit system;
      overlays = builtins.attrValues outputs.overlays;
      config = {
        allowUnfree = true;
        allowUnfreePredicate = _: true;
      };
      hostPlatform = system;
    };
  mkSystem =
    config:
    let
      hostname = config.hostname;
    in
    lib.nixosSystem {
      pkgs = mkPkgsWithSystem config.system;
      specialArgs = {
        inherit
          inputs
          outputs
          lib
          hostname
          ;
      };
      modules =
        lib.scanPath.toList { path = (lib.relativeToRoot "hosts/${hostname}"); }
        ++ (config.extraModules or [ ])
        ++ mkHomeUsers {
          users = config.homeUsers or [ ];
          hostname = config.hostname;
        };
    };
  mkHomeUsers =
    { users, hostname }:
    if users == [ ] then
      users
    else
      [
        {
          config.home-manager = {
            users = builtins.listToAttrs (
              builtins.map (name: {
                name = name;
                value = {
                  imports = [ (lib.relativeToRoot "home/${name}") ];
                  config.home = {
                    username = name;
                  };
                };
              }) users
            );
            backupFileExtension = "bk";
            useGlobalPkgs = true;
            useUserPackages = true;
            extraSpecialArgs = {
              inherit inputs outputs hostname;
            };
            sharedModules = [
              inputs.sops-nix.homeManagerModules.sops
            ];
          };
        }
      ];
in
{
  mkSystems =
    hosts:
    builtins.listToAttrs (
      builtins.map (config: {
        name = config.hostname;
        value = mkSystem config;
      }) hosts
    );
}
