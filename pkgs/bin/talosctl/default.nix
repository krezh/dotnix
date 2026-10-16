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
  version = "1.14.4";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${finalAttrs.version}";
    hash = "sha256-TGvT1Rowfn3xKss4Jzd1A90BwWC6yobPJZ0v/hQI6CI=";
  };

  vendorHash = "sha256-GKBzCvRhWTsRydQ7Te+YzQIoXzUNeuskzNWIwx85EnQ=";

  overrideModAttrs = _: {
    buildPhase = ''
      go work vendor
    '';
  };

  ldflags = [
    "-s"
    "-w"
  ];

  subPackages = [ "cmd/${finalAttrs.pname}" ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd ${finalAttrs.pname} \
      --bash <($out/bin/${finalAttrs.pname} completion bash) \
      --fish <($out/bin/${finalAttrs.pname} completion fish) \
      --zsh <($out/bin/${finalAttrs.pname} completion zsh)
  '';

  doCheck = false;

  doInstallCheck = true;

  installCheckPhase = ''
    $out/bin/${finalAttrs.pname} version --client | grep ${finalAttrs.version} > /dev/null
  '';

  meta = {
    description = "A CLI for out-of-band management of Kubernetes nodes created by Talos";
    homepage = "https://www.talos.dev/";
    changelog = "https://github.com/siderolabs/talos/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.mpl20;
    mainProgram = finalAttrs.pname;
  };
})
