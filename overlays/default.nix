# This file defines overlays
{ lib, inputs, ... }:
{
  # This one brings our custom packages from the 'packages' directory
  pkgs =
    final: _prev:
    import ../pkgs {
      pkgs = final;
      inherit lib;
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

  # Override swww to use flake input when available
  swww-flake = _final: prev: {
    swww = if (inputs ? swww) then inputs.swww.packages.${prev.system}.swww else prev.swww;
  };

  # Fix Weston DRM modifier assertion crash on AMD GPUs
  weston-fix = import ./weston-fix.nix;
}
