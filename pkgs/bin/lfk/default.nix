{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule (finalAttrs: {
  pname = "lfk";
  # renovate: datasource=github-releases depName=janosmiko/lfk
  version = "0.9.21";

  src = fetchFromGitHub {
    owner = "janosmiko";
    repo = "lfk";
    tag = "v${finalAttrs.version}";
    hash = "sha256-6qNpkWUTZKD+v0INn1ZNw1LJZaPdODnWCNhlE5hOv3s=";
  };

  postPatch = ''
    substituteInPlace go.mod \
      --replace "go 1.26.2" "go 1.26.1"
  '';

  vendorHash = "sha256-mx5IuJLGtNx2WZUfF/TdubwOGCr0Wjy7s2zvzOXqyO0=";
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
