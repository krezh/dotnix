{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-browse-pvc";
  # renovate: datasource=github-release depName=clbx/kubectl-browse-pvc
  version = "1.1.0";

  src = fetchFromGitHub {
    owner = "clbx";
    repo = "kubectl-browse-pvc";
    rev = "v${version}";
    hash = "sha256-BkLcS4FFSkYG+NQW956RaLievD+GdOMjQ62tIi6J+MM=";
  };

  sourceRoot = "${src.name}/src";

  ldflags = [
    "-s"
    "-w"
    "-X main.Version=${version}"
  ];

  vendorHash = "sha256-kalnhBWVZaStdUeTiKln0mVow4x1K2+BZPXG+5/YRVM=";

  meta = {
    description = "Kubectl plugin for browsing PVCs on the command line";
    homepage = "https://github.com/clbx/kubectl-browse-pvc";
    license = lib.licenses.mit;
    maintainers = [ ];
    mainProgram = pname;
    platforms = lib.platforms.all;
  };
}
