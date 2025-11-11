{
  inputs,
  outputs,
  lib,
  self,
  ...
}:

let
  # Load and merge all .nix files from vars/ directory using lib.scanPath
  var = builtins.foldl' lib.recursiveUpdate { } (
    map import (lib.scanPath.toList { path = lib.relativeToRoot "vars"; })
  );

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
    hostname: config:
    let
      homeUsers = config.homeUsers or [ ];
      desktop = config.desktop or true;
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
          var
          ;
      };
      modules =
        (getHostModules {
          inherit hostname;
          commonHost = config.commonHost or true;
        })
        ++ (config.extraModules or [ ])
        ++ mkHomeUsers {
          users = homeUsers;
          inherit hostname desktop;
        };
    };
  getHostModules =
    {
      hostname,
      commonHost ? true,
    }:
    let
      hostPath = lib.relativeToRoot "hosts/${hostname}";
      commonPath = lib.relativeToRoot "hosts/common";
      commonModules = if commonHost then [ (lib.scanPath.toImports commonPath) ] else [ ];
    in
    if lib.pathExists hostPath then commonModules ++ [ (lib.scanPath.toImports hostPath) ] else [ ];

  mkHomeUsers =
    {
      users,
      hostname,
      desktop ? true,
    }:
    lib.optionals (users != [ ]) [
      {
        config.home-manager = {
          users = lib.genAttrs users (name: {
            imports = [
              (lib.scanPath.toImports (lib.relativeToRoot "home/${name}/common"))
            ]
            ++ (
              if desktop then [ (lib.scanPath.toImports (lib.relativeToRoot "home/${name}/desktop")) ] else [ ]
            )
            ++ outputs.homeManagerModules;
            config.home.username = name;
          });
          backupFileExtension = "bk";
          useGlobalPkgs = true;
          useUserPackages = true;
          extraSpecialArgs = {
            inherit
              inputs
              outputs
              hostname
              var
              ;
          };
          sharedModules = [
            inputs.sops-nix.homeManagerModules.sops
          ];
        };
      }
    ];
  mapToGha =
    system:
    {
      "x86_64-linux" = "ubuntu-latest";
      "x86_64-darwin" = "ubuntu-latest";
      "aarch64-linux" = "ubuntu-24.04-arm";
    }
    .${system} or system;
in
{
  mkSystems = hosts: lib.mapAttrs (hostname: config: mkSystem hostname config) hosts;

  # Used by CI
  top = lib.genAttrs (builtins.attrNames self.nixosConfigurations) (
    attr: self.nixosConfigurations.${attr}.config.system.build.toplevel
  );

  # Lists hosts with their system kind for use in github actions
  ghMatrix =
    {
      exclude ? [ ],
    }:
    {
      include =
        builtins.map
          (host: {
            inherit host;
            system = self.nixosConfigurations.${host}.pkgs.stdenv.hostPlatform.system;
            runner = mapToGha self.nixosConfigurations.${host}.pkgs.stdenv.hostPlatform.system;
          })
          (builtins.filter (host: !builtins.elem host exclude) (builtins.attrNames self.nixosConfigurations));
    };
}
