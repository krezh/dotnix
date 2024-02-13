#{ pkgs ? import <nixpkgs> { } }:
# Custom packages, that can be defined similarly to ones from nixpkgs
# You can build them using 'nix build .#example'
pkgs:
{
  #shellcolord = pkgs.callPackage ./shellcolord { };
}
