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
  version = "0.7.1";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "flate";
    tag = "v${finalAttrs.version}";
    hash = "sha256-OqVZ974iJKwFDsKOWAeVW0QAsw+w5NeHwkBRAIqBfqs=";
  };

  doCheck = false;

  vendorHash = "sha256-W2py0nMMlYIP7JnEmqkfzVh0OqSKoWnRtkBYT3UeHIM=";

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
    mainProgram = "flate";
  };
})
