{ inputs, ... }:
let
  lib = inputs.nixpkgs.lib;
in
{
  toList =
    {
      path,
      excludeFiles ? [ ],
    }:
    builtins.map (f: (path + "/${f}")) (
      builtins.attrNames (
        lib.attrsets.filterAttrs (
          path: _type:
          !(builtins.elem path ([ "default.nix" ] ++ excludeFiles)) # ignore default.nix
          && (
            (_type == "directory") # include directories
            || (lib.strings.hasSuffix ".nix" path) # include .nix files
          )
        ) (builtins.readDir path)
      )
    );
  toAttrs =
    {
      paths,
      excludeFiles ? [ ],
      args ? { },
    }:
    let
      pathsList = if builtins.isList paths then paths else [ paths ];
    in
    builtins.listToAttrs (
      builtins.concatMap (
        p:
        builtins.map
          (f: {
            name = if lib.strings.hasSuffix ".nix" f then lib.strings.removeSuffix ".nix" f else f;
            value = p.func (p.path + "/${f}") args;
          })
          (
            builtins.attrNames (
              lib.attrsets.filterAttrs (
                path: _type:
                !(builtins.elem path ([ "default.nix" ] ++ excludeFiles)) # ignore default.nix
                && (
                  (_type == "directory") # include directories
                  || (lib.strings.hasSuffix ".nix" path) # include .nix files
                )
              ) (builtins.readDir p.path)
            )
          )
      ) pathsList
    );
}
