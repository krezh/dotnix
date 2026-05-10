{ pkgs, go-bin, ... }:
let
  src = builtins.path {
    path = ./src;
    name = "k8s-format-src";
  };
in
pkgs.buildGoApplication {
  pname = "k8s-format";
  version = "0.0.0";
  inherit src;

  go = go-bin.latestStable;
  modules = "${src}/govendor.toml";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    mainProgram = "k8s-format";
  };
}
