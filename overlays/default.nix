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

  # # When applied, the unstable nixpkgs set (declared in the flake inputs) will
  # # be accessible through 'pkgs.unstable'
  unstable-packages = final: _prev: {
    unstable = import inputs.nixpkgs-unstable {
      system = final.system;
      config.allowUnfree = true;
    };
  };
  overrideNix = _final: prev: {
    inherit (prev.lixPackageSets.stable)
      #nixpkgs-review #TODO Broken atm https://git.lix.systems/lix-project/lix/issues/980
      #nix-direnv #TODO Broken atm https://git.lix.systems/lix-project/lix/issues/980
      nix-eval-jobs
      nix-fast-build
      colmena
      ;
  };
}
