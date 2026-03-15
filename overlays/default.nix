{ lib, inputs, ... }:
{
  pkgs =
    final: _prev:
    lib.scanPath.toAttrs {
      basePath = lib.relativeToRoot "pkgs";
      func = final.callPackage;
      useBaseName = true;
    };

  go-overlay = inputs.go-overlay.overlays.default;

  nix4vscode = inputs.nix4vscode.overlays.default;

  kernel = _final: prev: {
    linux = prev.linux.overrideAttrs (old: {
      requiredSystemFeatures = (old.requiredSystemFeatures or [ ]) ++ [ "kernelbuild" ];
    });
  };

  lix = _final: prev: {
    inherit (prev.lixPackageSets.latest)
      # nixpkgs-review
      # nix-fast-build
      nix-eval-jobs
      colmena
      ;
  };

  # Fix Weston DRM modifier assertion crash on AMD GPUs
  weston-fix = import ./weston-fix.nix;
}
