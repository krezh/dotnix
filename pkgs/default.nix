# Custom packages, that can be defined similarly to ones from nixpkgs
# You can build them using 'nix build .#example' or (legacy) 'nix-build -A example'
{ pkgs, lib, ... }:
let
  inherit (pkgs) callPackage;
in
lib.mapPathsToAttrs {
  func = callPackage;
  path = [
    ./bin
    ./scripts
  ];
  args = { };
}
