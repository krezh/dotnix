# This file defines overlays
{
  lib,
  inputs,
  ...
}:
{
  # This one brings our custom packages from the 'packages' directory
  pkgs =
    final: _prev:
    lib.scanPath.toAttrs {
      basePath = lib.relativeToRoot "pkgs";
      func = final.callPackage;
      useBaseName = true;
    };

  go-overlay = inputs.go-overlay.overlays.default;

  nix4vscode = inputs.nix4vscode.overlays.default;

  lix = (
    _final: prev: {
      inherit (prev.lixPackageSets.latest)
        # nixpkgs-review
        nix-eval-jobs
        # nix-fast-build
        colmena
        ;
    }
  );

  # Fix Weston DRM modifier assertion crash on AMD GPUs
  weston-fix = import ./weston-fix.nix;
}
