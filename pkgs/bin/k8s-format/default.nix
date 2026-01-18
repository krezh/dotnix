{ pkgs, lib, ... }:
pkgs.buildGoApplication {
  pname = "k8s-format";
  version = "0.0.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = ./src;
  };
  modules = ./src/gomod2nix.toml;

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    mainProgram = "k8s-format";
  };
}
