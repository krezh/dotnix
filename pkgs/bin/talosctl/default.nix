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
  version = "1.13.8";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${finalAttrs.version}";
    hash = "sha256-uwcB0ibDLE1zDU9dCgh9THkcrmsAMwA93+Ihh7Sc4d4=";
  };

  vendorHash = "sha256-zQwvnfirUeN5w1FQu1wSR3pa9LJ3nurIliUVdZDCr8g=";

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
