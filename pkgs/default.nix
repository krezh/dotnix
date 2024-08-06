{ pkgs, lib, ... }:
lib.scanPath.toAttrs {
  func = pkgs.callPackage;
  path = [
    ./bin
    ./scripts
  ];
  args = { };
}
