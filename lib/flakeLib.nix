# Helper functions for modular NixOS and home-manager configuration.
{
  inputs,
  outputs,
  lib,
  ...
}:
let
  var = builtins.foldl' lib.recursiveUpdate { } (
    map import (lib.scanPath.toList { basePath = lib.relativeToRoot "vars"; })
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
      users = config.users or [ ];
      userProfiles = config.profiles or [ ];
      ci = config.ci or true;
    in
    if (users == [ ] || users == null) && userProfiles != [ ] && userProfiles != null then
      throw "Host '${hostname}': profiles cannot be set when users is empty or null"
    else
      lib.nixosSystem {
        pkgs = mkPkgsWithSystem config.system;
        specialArgs = {
          inherit
            inputs
            outputs
            lib
            hostname
            users
            var
            ;
        };
        modules =
          (getHostModules {
            inherit hostname users;
            includeCommon = config.includeCommon or true;
          })
          ++ (config.extraModules or [ ])
          ++ mkHomeUsers {
            inherit users;
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
      users ? [ ],
      includeCommon ? true,
    }:
    let
      hostPath = lib.relativeToRoot "hosts/${hostname}";
      commonPath = lib.relativeToRoot "hosts/common";
      commonUsersPath = lib.path.append commonPath "users";

      # Scan and filter common modules
      allCommonPaths = lib.scanPath.toList { basePath = commonPath; };

      # Filter function to exclude user directories if users is empty
      filteredCommonPaths =
        if users == [ ] || users == null then
          lib.filter (path: !(lib.hasPrefix (toString commonUsersPath) (toString path))) allCommonPaths
        else
          allCommonPaths;

      # Create imports structure
      commonImports = {
        imports = filteredCommonPaths;
      };
    in
    lib.optionals includeCommon [ commonImports ]
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
