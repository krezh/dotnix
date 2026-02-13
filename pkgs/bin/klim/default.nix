{ pkgs, go-bin, ... }:
pkgs.buildGoApplication rec {
  pname = "klim";
  version = "0.1.0";
  src = builtins.path {
    path = ./src;
    name = "klim-src";
  };

  go = go-bin.latestStable;
  modules = "${src}/govendor.toml";

  ldflags = [
    "-s"
    "-w"
    "-X main.version=${version}"
  ];
}
