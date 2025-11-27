# Helper functions for modular NixOS and home-manager configuration.
{
  inputs,
  outputs,
  lib,
  ...
}:

let
  var = builtins.foldl' lib.recursiveUpdate { } (
    map import (lib.scanPath.toList { path = lib.relativeToRoot "vars"; })
  );

  mkPkgsWithSystem =
    system:
    import inputs.nixpkgs {
      inherit system;
      overlays = builtins.attrValues outputs.overlays;
      config.allowUnfree = true;
      config.allowUnfreePredicate = _: true;
      hostPlatform = system;
    };

  mkSystem =
    hostname: config:
    let
      homeUsers = config.homeUsers or [ ];
      userProfiles = config.profiles or [ ];
      ci = config.ci or true;
    in
    if (homeUsers == [ ] || homeUsers == null) && userProfiles != [ ] && userProfiles != null then
      throw "Host '${hostname}': profiles cannot be set when homeUsers is empty or null"
    else
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
            includeCommon = config.includeCommon or true;
          })
          ++ (config.extraModules or [ ])
          ++ mkHomeUsers {
            users = homeUsers;
            profiles = userProfiles;
            inherit hostname;
          };
      }
      // {
        inherit ci;
      };

  getHostModules =
    {
      hostname,
      includeCommon ? true,
    }:
    let
      hostPath = lib.relativeToRoot "hosts/${hostname}";
      commonPath = lib.relativeToRoot "hosts/common";
    in
    lib.optionals includeCommon [ (lib.scanPath.toImports commonPath) ]
    ++ lib.optionals (builtins.pathExists hostPath) [ (lib.scanPath.toImports hostPath) ];

  mkHomeUsers =
    {
      users,
      hostname,
      profiles ? [ ],
    }:
    lib.optionals (users != [ ]) [
      {
        config.home-manager = {
          users = lib.genAttrs users (
            name:
            let
              userBase = lib.relativeToRoot "home/${name}";
              basePath = lib.path.append userBase "base";
              hostProfilePath = lib.path.append userBase hostname;

              inclusions = lib.filter (p: !(lib.hasPrefix "!" p)) profiles;
              exclusions = map (lib.removePrefix "!") (lib.filter (lib.hasPrefix "!") profiles);

              profilePaths = lib.filter (
                path:
                builtins.pathExists path && !(builtins.elem path (map (p: lib.path.append userBase p) exclusions))
              ) (map (p: lib.path.append userBase p) inclusions);
            in
            {
              imports = [
                (lib.scanPath.toImports basePath)
              ]
              ++ map lib.scanPath.toImports profilePaths
              ++ lib.optionals (builtins.pathExists hostProfilePath) [ (lib.scanPath.toImports hostProfilePath) ]
              ++ outputs.homeManagerModules;
              config.home.username = name;
            }
          );
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
          sharedModules = [ inputs.sops-nix.homeManagerModules.sops ];
        };
      }
    ];

in
{
  mkSystems = lib.mapAttrs mkSystem;
}
