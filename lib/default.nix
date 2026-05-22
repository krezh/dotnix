{ inputs, ... }:
inputs.nixpkgs.lib.extend (
  _final: prev: {
    relativeToRoot = prev.path.append ../.;
    scanPath = import ./scanPath.nix { lib = prev; };
    processModules =
      modules:
      let
        prefixes = prev.unique (
          prev.concatMap (
            name:
            let
              parts = prev.splitString "-" name;
            in
            if builtins.length parts > 1 then [ (builtins.head parts) ] else [ ]
          ) (prev.attrNames modules)
        );
        groups = prev.filterAttrs (name: _: !(modules ? ${name})) (
          prev.genAttrs prefixes (prefix: {
            imports = prev.attrValues (prev.filterAttrs (name: _: prev.hasPrefix "${prefix}-" name) modules);
          })
        );
      in
      modules // groups;
  }
)
