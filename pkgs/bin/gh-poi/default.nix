{ buildGoModule, fetchFromGitHub }:
buildGoModule rec {
  pname = "gh-poi";
  # renovate: datasource=github-releases depName=seachicken/gh-poi
  version = "0.14.0";

  src = fetchFromGitHub {
    owner = "seachicken";
    repo = "gh-poi";
    rev = "v${version}";
    hash = "sha256-bbmNzxGRg7nKfB8xu90ZkKrhWwY24G6h8TW07f9IpTY=";
  };

  vendorHash = "sha256-ciOJpVqSPJJLX/sqrztqB3YSoMUrEnn52gGddE80rV0=";

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false;
}
