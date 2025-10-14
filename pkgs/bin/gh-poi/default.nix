{ buildGoModule, fetchFromGitHub }:
buildGoModule rec {
  pname = "gh-poi";
  # renovate: datasource=github-releases depName=seachicken/gh-poi
  version = "0.14.1";

  src = fetchFromGitHub {
    owner = "seachicken";
    repo = "gh-poi";
    rev = "v${version}";
    hash = "sha256-HwFmSeDPpX1zbJh+0laekphmpnAsEdFBhgoLfT7CCYY=";
  };

  vendorHash = "sha256-ciOJpVqSPJJLX/sqrztqB3YSoMUrEnn52gGddE80rV0=";

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false;
}
