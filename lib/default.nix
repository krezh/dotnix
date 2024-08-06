{ lib, ... }:
{
  scanPath = import ./scanPath.nix { inherit lib; };

  relativeToRoot = lib.path.append ../.;
}
