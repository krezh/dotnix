{ buildGoModule, fetchFromGitHub }:
buildGoModule rec {
  pname = "gh-poi";
  # renovate: datasource=github-releases depName=seachicken/gh-poi
  version = "0.13.0";

  src = fetchFromGitHub {
    owner = "seachicken";
    repo = "gh-poi";
    rev = "v${version}";
    hash = "sha256-foUv6+QIfPlYwgTwxFvEgGeOw/mpC80+ntHo29LQbB8=";
  };

  vendorHash = "sha256-D/YZLwwGJWCekq9mpfCECzJyJ/xSlg7fC6leJh+e8i0=";

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false;
}
