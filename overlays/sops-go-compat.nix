# sops-nix pins buildGo125Module, which nixpkgs dropped once Go 1.25 went EOL.
# Restore it on the same latest-stable toolchain every other Go package here uses.
# Drop once https://github.com/Mic92/sops-nix/issues/983 lands.
final: prev: {
  buildGo125Module = prev.buildGoModule.override { go = final.go-bin.latestStable; };
}
