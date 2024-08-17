{ inputs, ... }:
{
  toList =
    {
      path,
      excludeFiles ? [ ],
    }:
    builtins.map (f: (path + "/${f}")) (
      builtins.attrNames (
        inputs.nixpkgs.lib.attrsets.filterAttrs (
          path: _type:
          !(builtins.elem path ([ "default.nix" ] ++ excludeFiles)) # ignore default.nix
          && (
            (_type == "directory") # include directories
            || (inputs.nixpkgs.lib.strings.hasSuffix ".nix" path) # include .nix files
          )
        ) (builtins.readDir path)
      )
    );
  toAttrs =
    {
      func ? { },
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
            name =
              if inputs.nixpkgs.lib.strings.hasSuffix ".nix" f then
                inputs.nixpkgs.lib.strings.removeSuffix ".nix" f
              else
                f;
            value = func (p + "/${f}") args;
          })
          (
            builtins.attrNames (
              inputs.nixpkgs.lib.attrsets.filterAttrs (
                path: _type:
                !(builtins.elem path ([ "default.nix" ] ++ excludeFiles)) # ignore default.nix
                && (
                  (_type == "directory") # include directories
                  || (inputs.nixpkgs.lib.strings.hasSuffix ".nix" path) # include .nix files
                )
              ) (builtins.readDir p)
            )
          )
      ) paths
    );
}
