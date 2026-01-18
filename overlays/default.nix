# This file defines overlays
{ lib, inputs, ... }:
{
  # This one brings our custom packages from the 'packages' directory
  pkgs =
    final: _prev:
    lib.scanPath.toAttrs {
      basePath = lib.relativeToRoot "pkgs";
      func = final.callPackage;
      useBaseName = true;
      excludeFiles = [ "vscode-extensions" ];
    };

  gomod2nix = inputs.gomod2nix.overlays.default;

  nix4vscode = inputs.nix4vscode.overlays.default;

  # Fix Weston DRM modifier assertion crash on AMD GPUs
  weston-fix = import ./weston-fix.nix;
}
