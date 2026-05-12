{
  buildGoModule,
  fetchFromGitHub,
  fetchzip,
  go-bin,
  installShellFiles,
  lib,
  stdenv,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "fluxcd";
  # renovate: datasource=github-releases depName=fluxcd/flux2
  version = "2.8.7";

  src = fetchFromGitHub {
    owner = "fluxcd";
    repo = "flux2";
    rev = "v${finalAttrs.version}";
    hash = "sha256-aA4DA/uG/YZ/9cUWAhhVZJme8Cj7OWFWqXce8n5InfM=";
  };

  vendorHash = "sha256-kn0S5tycL6tpgy/0YrHSFM0WpUluugSy87rh1FArnO8=";

  manifests = fetchzip {
    url = "https://github.com/fluxcd/flux2/releases/download/v${finalAttrs.version}/manifests.tar.gz";
    hash = "sha256-PdhR+UDquIJWtpSymtT6V7qO5fVJOkFz6RGzAx7xeb4=";
    stripRoot = false;
  };

  postUnpack = ''
    cp -r ${finalAttrs.manifests} source/cmd/flux/manifests
    rm source/cmd/flux/create_secret_git_test.go
    rm -f source/cmd/flux/install_test.go
  '';

  ldflags = [
    "-s"
    "-w"
    "-X main.VERSION=${finalAttrs.version}"
  ];

  subPackages = [ "cmd/flux" ];

  HOME = "$TMPDIR";

  nativeBuildInputs = [ installShellFiles ];

  doInstallCheck = true;
  installCheckPhase = ''
    $out/bin/flux --version | grep ${finalAttrs.version} > /dev/null
  '';

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    for shell in bash fish zsh; do
      $out/bin/flux completion $shell > flux.$shell
      installShellCompletion flux.$shell
    done
  '';

  meta = {
    changelog = "https://github.com/fluxcd/flux2/releases/tag/v${finalAttrs.version}";
    description = "Open and extensible continuous delivery solution for Kubernetes";
    homepage = "https://fluxcd.io";
    license = lib.licenses.asl20;
    mainProgram = "flux";
  };
})
