# This file defines overlays
{ lib, inputs, ... }:
{
  # This one brings our custom packages from the 'packages' directory
  pkgs =
    final: _prev:
    lib.scanPath.toAttrs {
      path = ../pkgs;
      func = final.callPackage;
      useBaseName = true;
    };

  overrideNix =
    _final: prev:
    let
      lix = prev.lixPackageSets.stable;
    in
    {
      inherit (lix) nix-eval-jobs colmena;
      nix-fast-build = prev.nix-fast-build.override { nix-eval-jobs = lix.nix-eval-jobs; };
    };

  gomod2nix = inputs.gomod2nix.overlays.default;

  # Fix Weston DRM modifier assertion crash on AMD GPUs
  weston-fix = import ./weston-fix.nix;
}
