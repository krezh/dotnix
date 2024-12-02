# This file defines overlays
{ lib, inputs, ... }:
{
  # This one brings our custom packages from the 'packages' directory
  additions =
    final: _prev:
    import ../pkgs {
      pkgs = final;
      inherit lib;
    };

  # This one contains whatever you want to overlay
  # You can change versions, add patches, set compilation flags, anything really.
  # https://nixos.wiki/wiki/Overlays
  modifications = _final: _prev: {
    # example = prev.example.overrideAttrs (oldAttrs: rec {
    # ...
    # });
  };

  # # When applied, the unstable nixpkgs set (declared in the flake inputs) will
  # # be accessible through 'pkgs.unstable'
  unstable-packages = final: _prev: {
    unstable = import inputs.nixpkgs-unstable {
      system = final.system;
      config.allowUnfree = true;
    };
  };
}
