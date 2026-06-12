{
  lib,
  stdenv,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  installShellFiles,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "talosctl";
  # renovate: datasource=github-releases depName=siderolabs/talos
  version = "1.13.4";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${finalAttrs.version}";
    hash = "sha256-jcBRAixpdfbzOfCc7BjyDScWPsrW9Mc7A8vGLyN1f68=";
  };

  vendorHash = "sha256-kfaqywGr6dF1k7tFtNBb+g6dycsV1B9g7a2Bu9qjocA=";

  ldflags = [
    "-s"
    "-w"
  ];

  env.GOWORK = "off";

  subPackages = [ "cmd/talosctl" ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd talosctl \
      --bash <($out/bin/talosctl completion bash) \
      --fish <($out/bin/talosctl completion fish) \
      --zsh <($out/bin/talosctl completion zsh)
  '';

  doCheck = false;

  doInstallCheck = true;

  installCheckPhase = ''
    $out/bin/talosctl version --client | grep ${finalAttrs.version} > /dev/null
  '';

  meta = {
    description = "A CLI for out-of-band management of Kubernetes nodes created by Talos";
    homepage = "https://www.talos.dev/";
    license = lib.licenses.mpl20;
    mainProgram = finalAttrs.pname;
  };
})
