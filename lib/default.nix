{ nixpkgs, ... }:
{
  scanPath =
    {
      path,
      excludeFiles ? [ ],
    }:
    builtins.map (f: (path + "/${f}")) (
      builtins.attrNames (
        nixpkgs.lib.attrsets.filterAttrs (
          path: _type:
          !(builtins.elem path ([ "default.nix" ] ++ excludeFiles)) # ignore default.nix
          && (
            (_type == "directory") # include directories
            || (nixpkgs.lib.strings.hasSuffix ".nix" path) # include .nix files
          )
        ) (builtins.readDir path)
      )
    );
}
