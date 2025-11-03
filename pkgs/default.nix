{ pkgs, lib, ... }:
# Auto-discover and merge packages from all directories
lib.discoverPackages pkgs.callPackage [
  ./bin
  ./scripts
]
