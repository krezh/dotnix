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
  version = "0.1.4";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "flate";
    tag = finalAttrs.version;
    hash = "sha256-wKN8ZWVpaZIa8idy0ZSAFyG4t6N5RGmc1S4VhfBhY7A=";
  };

  doCheck = false;

  vendorHash = "sha256-ZkA4iE/cc7IhTWpp+2JC6V1P0CUFVMdBL/F2whtbGTM=";

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
