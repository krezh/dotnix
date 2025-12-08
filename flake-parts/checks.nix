{
  self,
  lib,
  ...
}:
{
  perSystem =
    { system, ... }:
    {
      checks =
        let
          # Add all packages as checks
          packageChecks = lib.mapAttrs' (
            name: pkg: lib.nameValuePair "package-${name}" pkg
          ) self.packages.${system};

          # Add NixOS configurations as checks (only for systems that match current system)
          nixosChecks =
            lib.mapAttrs' (name: config: lib.nameValuePair "nixos-${name}" config.config.system.build.toplevel)
              (
                lib.filterAttrs (
                  _name: config: config.pkgs.stdenv.hostPlatform.system == system
                ) self.nixosConfigurations
              );
        in
        packageChecks // nixosChecks;
    };
}
