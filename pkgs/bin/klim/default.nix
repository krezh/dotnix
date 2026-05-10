{
  pkgs,
  go-bin,
  lib,
  ...
}:
let
  version = "0.1.0";
  src = builtins.path {
    path = ./src;
    name = "klim-src";
  };
in
pkgs.buildGoApplication {
  pname = "klim";
  inherit version src;

  go = go-bin.latestStable;
  modules = "${src}/govendor.toml";

  ldflags = [
    "-s"
    "-w"
    "-X main.version=${version}"
  ];

  meta = {
    description = "Analyzes Kubernetes cluster resource usage and provides optimization recommendations";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.mit;
    mainProgram = "klim";
  };
}
