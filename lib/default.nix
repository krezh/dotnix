{ nixpkgs, ... }:
{
  listNixFiles =
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
  mapPathsToAttrs =
    {
      func,
      path,
      excludeFiles ? [ ],
      args ? { },
    }:
    let
      paths = if builtins.isList path then path else [ path ];
    in
    builtins.listToAttrs (
      builtins.concatMap (
        p:
        builtins.map
          (f: {
            name = f;
            value = func (p + "/${f}") args;
          })
          (
            builtins.attrNames (
              nixpkgs.lib.attrsets.filterAttrs (
                path: _type:
                !(builtins.elem path ([ "default.nix" ] ++ excludeFiles)) # ignore default.nix
                && (
                  (_type == "directory") # include directories
                  || (nixpkgs.lib.strings.hasSuffix ".nix" path) # include .nix files
                )
              ) (builtins.readDir p)
            )
          )
      ) paths
    );
}
