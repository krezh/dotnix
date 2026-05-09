{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "kubectl-pgo";
  # renovate: datasource=github-releases depName=CrunchyData/postgres-operator-client
  version = "0.5.3";

  src = fetchFromGitHub {
    owner = "CrunchyData";
    repo = "postgres-operator-client";
    rev = "v${finalAttrs.version}";
    hash = "sha256-m8k4BiZx6ILUFYgpeXD2/Qy8HyBf/C51ErOy19baMhI=";
    fetchSubmodules = true;
  };

  vendorHash = "sha256-2w3pccBAYwj1ucEAIr+31xWdxJBz3P9HrsIamTmBJXU=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "A kubectl plugin for managing PostgreSQL clusters with PGO";
    homepage = "https://github.com/CrunchyData/postgres-operator-client";
    license = lib.licenses.asl20;
    mainProgram = finalAttrs.pname;
  };
})
