{
  pkgs,
  go-bin,
  ...
}:
pkgs.buildGoApplication rec {
  pname = "k8s-format";
  version = "0.0.0";
  src = builtins.path {
    path = ./src;
    name = "k8s-format-src";
  };

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
