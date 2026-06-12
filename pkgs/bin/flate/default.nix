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
  version = "0.4.5";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "flate";
    tag = "v${finalAttrs.version}";
    hash = "sha256-zaaXHKGuxPrGGas4lRykg3KwzLvyNC1C1lcrwx9xmWE=";
  };

  doCheck = false;

  vendorHash = "sha256-y3C3AqZvQM5dHSUEStRjJvIbCWD6HeSH8v8j4NMn7jg=";

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
