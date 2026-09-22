{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:

(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "yayamlls";
  # renovate: datasource=github-releases depName=home-operations/yayamlls
  version = "0.3.1";
  __structuredAttrs = true;

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "yayamlls";
    tag = finalAttrs.version;
    hash = "sha256-0WL7qC5gLIMkjwnZ3lD1tD91ky0A5wG84EphLWe2wWc=";
  };

  vendorHash = "sha256-cHOVyjlY+xZ/4mcR//GCfhc/yCQrWxru368NLyTi5ko=";
  doCheck = false;

  ldflags = [
    "-s"
    "-w"
    "-X=main.version=${finalAttrs.version}"
    "-X=main.commit=${finalAttrs.src.rev}"
  ];

  meta = {
    description = "YAML language server in Go. Schema-driven diagnostics, completion, hover; pluggable rendering for Flux HelmRelease and Kustomization via flate";
    homepage = "https://github.com/home-operations/yayamlls";
    changelog = "https://github.com/home-operations/yayamlls/blob/${finalAttrs.src.rev}/CHANGELOG.md";
    license = lib.licenses.agpl3Only;
    mainProgram = "yayamlls";
  };
})
