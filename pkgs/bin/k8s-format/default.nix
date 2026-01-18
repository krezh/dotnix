{ pkgs, ... }:
pkgs.buildGoApplication rec {
  pname = "k8s-format";
  version = "0.0.0";
  src = builtins.path {
    path = ./src;
    name = "k8s-format-src";
  };
  modules = "${src}/gomod2nix.toml";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    mainProgram = "k8s-format";
  };
}
