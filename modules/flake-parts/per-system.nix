{ inputs, lib, ... }:
{
  perSystem =
    {
      pkgs,
      system,
      config,
      ...
    }:
    {
      # needed by gomod2nix
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [ inputs.self.overlays.default ];
      };
      packages =
        (lib.scanPath.toAttrs {
          basePath = lib.relativeToRoot "pkgs";
          func = pkgs.callPackage;
          useBaseName = true;
        })
        // {
          treefmt = config.treefmt.build.wrapper; # Expose treefmt wrapper to prevent GC
        };
    };
}
