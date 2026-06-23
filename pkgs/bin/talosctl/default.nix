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
  version = "1.13.5";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${finalAttrs.version}";
    hash = "sha256-woMLG4m7snKD3naTZWYEu78zC/eK5lDxd+uLyXdkzMo=";
  };

  vendorHash = "sha256-98jQJ7M/3ki5L6YQAxtk3bBnixfXhLX4WXY7DN4hsQ4=";

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
    license = lib.licenses.mpl20;
    mainProgram = finalAttrs.pname;
  };
})
