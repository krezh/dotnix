# Custom packages, that can be defined similarly to ones from nixpkgs
# You can build them using 'nix build .#example'

{ pkgs ? import <nixpkgs> { } }: {
  talosctl = pkgs.unstable.callPackage ./talosctl { };
  k9s = pkgs.unstable.callPackage ./k9s { };
}
