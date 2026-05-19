{
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  lib,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "zli";
  # renovate: datasource=github-releases depName=project-zot/zot
  version = "2.1.17";

  src = fetchFromGitHub {
    owner = "project-zot";
    repo = "zot";
    rev = "v${finalAttrs.version}";
    hash = "sha256-/1QEMpDq8okaVWhaynlJ+tE1b6AObUnHfHrmnylBKL0=";
  };

  vendorHash = "sha256-09LQKBKyqpgBbC44VPsZ3RJcwrHWy6TpF87u35UgcYI=";

  subPackages = [ "cmd/zli" ];

  tags = [ "search" ];

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false;

  meta = {
    description = "CLI client for the Zot OCI container registry";
    homepage = "https://zotregistry.dev";
    license = lib.licenses.asl20;
    mainProgram = "zli";
  };
})
