{
  inputs,
  lib,
  self,
  ...
}:
{
  perSystem =
    {
      pkgs,
      system,
      ...
    }:
    {
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = builtins.attrValues (import ../overlays { inherit inputs lib; });
        config = { };
      };

      packages = lib.scanPath.toAttrs {
        path = ../pkgs;
        func = pkgs.callPackage;
        useBaseName = true;
      };

      checks =
        let
          # Filter nixosConfigurations by current system and ci flag
          hostsForSystem = lib.filterAttrs (
            _name: config: (config.ci or true) && config.pkgs.stdenv.hostPlatform.system == system
          ) self.nixosConfigurations;
          # Map to toplevel derivations
          hostToplevels = lib.mapAttrs (_: config: config.config.system.build.toplevel) hostsForSystem;
        in
        hostToplevels // self.packages.${system} // (self.devShells.${system} or { });
    };
}
