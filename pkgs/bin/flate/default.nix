{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  stdenv,
  installShellFiles,
}:

(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "flate";
  # renovate: datasource=github-releases depName=home-operations/flate
  version = "0.1.21";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "flate";
    tag = finalAttrs.version;
    hash = "sha256-Tdlxw333Ri397BbjFSAcqPNea5edbqC8ysgqxlkAoKI=";
  };

  doCheck = false;

  vendorHash = "sha256-mka8xaPFbfS8OHT/kFCDTL0/DrCpEt5RDqTMWCjf3Bo=";

  ldflags = [
    "-s"
    "-w"
  ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd flate \
      --bash <($out/bin/flate completion bash) \
      --fish <($out/bin/flate completion fish) \
      --zsh <($out/bin/flate completion zsh)
  '';

  meta = {
    description = "Flate - A Flux resource inflator";
    homepage = "https://github.com/home-operations/flate";
    changelog = "https://github.com/home-operations/flate/blob/${finalAttrs.src.rev}/CHANGELOG.md";
    license = lib.licenses.agpl3Only;
    maintainers = with lib.maintainers; [ ];
    mainProgram = "flate";
  };
})
