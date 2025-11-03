{
  inputs,
  outputs,
  lib,
  self,
  ...
}:

let
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
        (getHostModules {
          inherit hostname;
          importCommon = config.importCommon or true;
        })
        ++ (config.extraModules or [ ])
        ++ mkHomeUsers {
          users = homeUsers;
          inherit hostname;
        };
    };
  getHostModules =
    {
      hostname,
      importCommon ? true,
    }:
    let
      hostPath = lib.relativeToRoot "hosts/${hostname}";
      commonPath = lib.relativeToRoot "hosts/common";
      commonModules = if importCommon then [ (lib.importTree commonPath) ] else [ ];
    in
    if lib.pathExists hostPath then
      # Use import-tree for common modules (unless disabled), plus import-tree for host-specific modules
      commonModules ++ [ (lib.importTree hostPath) ]
    else
      [ ];

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
