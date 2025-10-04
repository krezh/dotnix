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

  overrideNix = _final: prev: {
    inherit (prev.lixPackageSets.stable)
      nix-direnv
      nix-eval-jobs
      nix-fast-build
      colmena
      ;
  };

  # Override swww to use flake input when available
  swww-flake = _final: prev: {
    swww = if (inputs ? swww) then inputs.swww.packages.${prev.system}.swww else prev.swww;
  };
}
