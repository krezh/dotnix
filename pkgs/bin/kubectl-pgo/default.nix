{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-pgo";
  # renovate: datasource=github-tag depName=CrunchyData/postgres-operator-client
  version = "0.5.2";

  src = fetchFromGitHub {
    owner = "CrunchyData";
    repo = "postgres-operator-client";
    rev = "v${version}";
    hash = "sha256-6Kg+P7UEsbe9vmX0G1mlG89RfXXLBsXgcPMhzn5kbq4=";
    fetchSubmodules = true;
  };

  vendorHash = "sha256-T0rdqBeLKplmEwzchZWdhvU30G6u/vwzu3lQ2FO3+3U=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "";
    homepage = "https://github.com/CrunchyData/postgres-operator-client";
    license = lib.licenses.asl20;
    maintainers = [ ];
    mainProgram = pname;
  };
}
