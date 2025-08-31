{
  inputs,
  outputs,
  lib,
  self,
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

  # Used by CI
  top = lib.genAttrs (builtins.attrNames self.nixosConfigurations) (
    attr: self.nixosConfigurations.${attr}.config.system.build.toplevel
  );

  # Lists hosts with their system kind for use in github actions
  evalHosts = {
    include = builtins.map (host: {
      inherit host;
      system = self.nixosConfigurations.${host}.pkgs.system;
      runner = lib.mapToGha self.nixosConfigurations.${host}.pkgs.system;
    }) (builtins.attrNames self.nixosConfigurations);
  };
}
