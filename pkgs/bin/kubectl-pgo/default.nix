{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-pgo";
  # renovate: datasource=github-releases depName=CrunchyData/postgres-operator-client
  version = "0.5.3";

  src = fetchFromGitHub {
    owner = "CrunchyData";
    repo = "postgres-operator-client";
    rev = "v${version}";
    hash = "sha256-m8k4BiZx6ILUFYgpeXD2/Qy8HyBf/C51ErOy19baMhI=";
    fetchSubmodules = true;
  };

  vendorHash = "sha256-2w3pccBAYwj1ucEAIr+31xWdxJBz3P9HrsIamTmBJXU=";

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
