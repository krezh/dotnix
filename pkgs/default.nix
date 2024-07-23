# Custom packages, that can be defined similarly to ones from nixpkgs
# You can build them using 'nix build .#example' or (legacy) 'nix-build -A example'
{
  pkgs ? (import ../nixpkgs.nix) { },
  ...
}:
let
  inherit (pkgs) callPackage;
in
{
  talosctl = callPackage ./talosctl { };
  fluxcd = callPackage ./fluxcd { };
  brightness_script = callPackage ./brightness_script { };
  volume_script = callPackage ./volume_script { };
  volume_brightness = callPackage ./volume_brightness { };
}
