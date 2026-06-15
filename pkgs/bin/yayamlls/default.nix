{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:

(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "yayamlls";
  # renovate: datasource=github-releases depName=home-operations/yayamlls
  version = "0.1.8";
  __structuredAttrs = true;

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "yayamlls";
    tag = finalAttrs.version;
    hash = "sha256-uRS1Nyv2rEGc103vkpUOtjtqV3CDPUqQVk3tgqxjVwk=";
  };

  vendorHash = "sha256-M02zGmNjB86qUTSEXXG29gFXXmNRQlvxc6KbP4BUhqg=";
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
