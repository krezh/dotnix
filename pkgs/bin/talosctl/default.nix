{
  lib,
  pkgs,
}:
(pkgs.buildGoModule.override { go = pkgs.go-bin.latestStable; }) (finalAttrs: {
  pname = "talosctl";
  # renovate: datasource=github-releases depName=siderolabs/talos
  version = "1.14.7";

  src = pkgs.fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${finalAttrs.version}";
    hash = "sha256-Jf+wyrMXejK4gSsGJNMcCBnN9NjjPkvcx4q0YQ8fGC4=";
  };

  vendorHash = "sha256-jNii0LfgsVizzirRM2lByX4lXuGCORmnEahhLebq+Ow=";

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

  nativeBuildInputs = [ pkgs.installShellFiles ];

  postInstall = lib.optionalString (pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform) ''
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
