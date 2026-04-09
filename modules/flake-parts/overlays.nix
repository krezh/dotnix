{ inputs, lib, ... }:
{
  flake.overlays.default = lib.composeManyExtensions (
    (map import (
      lib.scanPath.toList {
        basePath = lib.relativeToRoot "overlays";
      }
    ))
    ++ [
      inputs.go-overlay.overlays.default
      inputs.nix4vscode.overlays.default
      (final: _prev: { craneLib = inputs.crane.mkLib final; })
      (
        final: _prev:
        lib.scanPath.toAttrs {
          basePath = lib.relativeToRoot "pkgs";
          func = final.callPackage;
          useBaseName = true;
        }
      )
    ]
  );
}
