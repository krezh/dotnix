{ pkgs, ... }:
pkgs.buildGoApplication {
  pname = "k8s-format";
  version = "0.0.0";
  src = ./src;
  modules = ./src/gomod2nix.toml;

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    mainProgram = "k8s-format";
  };
}
