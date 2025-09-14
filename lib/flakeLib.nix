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
      homeUsers = config.homeUsers or [ ];
    in
    lib.nixosSystem {
      pkgs = mkPkgsWithSystem config.system;
      specialArgs = {
        inherit
          inputs
          outputs
          lib
          hostname
          homeUsers
          ;
      };
      modules =
        (getHostModules hostname)
        ++ (config.extraModules or [ ])
        ++ mkHomeUsers {
          users = homeUsers;
          inherit hostname;
        };
    };
  getHostModules =
    hostname:
    let
      hostPath = lib.relativeToRoot "hosts/${hostname}";
    in
    if lib.pathExists hostPath then [ hostPath ] else [ ];

  mkHomeUsers =
    { users, hostname }:
    lib.optionals (users != [ ]) [
      {
        config.home-manager = {
          users = lib.genAttrs users (name: {
            imports = [ (lib.relativeToRoot "home/${name}") ];
            config.home.username = name;
          });
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
    lib.genAttrs (map (c: c.hostname) hosts) (
      hostname: mkSystem (lib.findFirst (c: c.hostname == hostname) null hosts)
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
