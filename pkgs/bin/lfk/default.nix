{
  lib,
  go-bin,
  buildGoModule,
  fetchFromGitHub,
}:

(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "lfk";
  # renovate: datasource=github-releases depName=janosmiko/lfk
  version = "0.17.4";

  src = fetchFromGitHub {
    owner = "janosmiko";
    repo = "lfk";
    tag = "v${finalAttrs.version}";
    hash = "sha256-WgcgXRSN1YcNORaL/MK+GI47+Uf/VUjayAvRBAF6FHA=";
  };

  vendorHash = "sha256-6ggE3voXdV8Yfv9fzdXFTI2zVPMfdPy6D8bWASRMc1g=";
  doCheck = false;

  ldflags = [
    "-s"
    "-w"
    "-X=github.com/janosmiko/lfk/internal/version.Version=${finalAttrs.version}"
    "-X=github.com/janosmiko/lfk/internal/version.GitCommit=${finalAttrs.src.rev}"
    "-X=github.com/janosmiko/lfk/internal/version.BuildDate=1970-01-01T00:00:00Z"
  ];

  meta = {
    description = "LFK is a lightning-fast, keyboard-focused, yazi-inspired terminal user interface for navigating and managing Kubernetes clusters. Built for speed and efficiency, it brings a three-column Miller columns layout with an owner-based resource hierarchy to your terminal";
    homepage = "https://github.com/janosmiko/lfk";
    license = lib.licenses.asl20;
    mainProgram = "lfk";
  };
})
