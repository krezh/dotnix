{
  inputs,
  ...
}:
let
  lib = inputs.nixpkgs.lib // import ../lib { inherit inputs; };
in
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

      packages = import ../pkgs { inherit pkgs lib; };
    };
}
