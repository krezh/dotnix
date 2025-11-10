{
  inputs,
  lib,
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
    };
}
