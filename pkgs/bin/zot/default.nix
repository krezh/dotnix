{
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  lib,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "zot";
  # renovate: datasource=github-releases depName=project-zot/zot
  version = "2.1.17";

  src = fetchFromGitHub {
    owner = "project-zot";
    repo = "zot";
    rev = "v${finalAttrs.version}";
    hash = "sha256-/1QEMpDq8okaVWhaynlJ+tE1b6AObUnHfHrmnylBKL0=";
  };

  vendorHash = "sha256-09LQKBKyqpgBbC44VPsZ3RJcwrHWy6TpF87u35UgcYI=";

  subPackages = [ "cmd/zot" ];

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false;

  meta = {
    description = "Production-ready vendor-neutral OCI-native container image registry";
    homepage = "https://zotregistry.dev";
    license = lib.licenses.asl20;
    mainProgram = "zot";
  };
})
