# Configures per-system Nix package environments and custom packages
{
  inputs,
  lib,
  ...
}: {
  perSystem = {
    pkgs,
    system,
    ...
  }: {
    # Provides the nixpkgs package set with system-specific overlays applied
    # needed by gomod2nix
    _module.args.pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = builtins.attrValues (import (lib.relativeToRoot "overlays") {inherit inputs lib;});
    };

    # Automatically discovers all packages from the pkgs directory
    packages = lib.scanPath.toAttrs {
      basePath = lib.relativeToRoot "pkgs";
      func = pkgs.callPackage;
      useBaseName = true;
    };
  };
}
