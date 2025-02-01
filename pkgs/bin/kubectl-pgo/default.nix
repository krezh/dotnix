{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-pgo";
  # renovate: datasource=github-tag depName=CrunchyData/postgres-operator-client
  version = "0.5.1";

  src = fetchFromGitHub {
    owner = "CrunchyData";
    repo = "postgres-operator-client";
    rev = "v${version}";
    hash = "sha256-Oj6coW7i4v4ovr6Cn8b41Pnrxik7QONuVuvWd22X0Ro=";
    fetchSubmodules = true;
  };

  vendorHash = "sha256-PhTDqsFhPas2mcK7Ew2TQNqnvftk/+7wo2yFE9dnSUY=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "";
    homepage = "https://github.com/CrunchyData/postgres-operator-client";
    license = lib.licenses.asl20;
    maintainers = [ ];
    mainProgram = "kubectl-pgo";
  };
}
