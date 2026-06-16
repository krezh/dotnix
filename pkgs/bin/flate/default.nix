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
  version = "0.4.8";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "flate";
    tag = "v${finalAttrs.version}";
    hash = "sha256-dNwQXM3KeMRBhwfKsPaaJa+LsW5FQ0zTta+tfDVR7Oo=";
  };

  doCheck = false;

  vendorHash = "sha256-13YLciazs2yRd7mFMfH5XkATdOG9H9scBTznfOS/I+M=";

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
