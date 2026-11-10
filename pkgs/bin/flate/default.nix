{
  lib,
  pkgs,
}:

(pkgs.buildGoModule.override { go = pkgs.go-bin.latestStable; }) (finalAttrs: {
  pname = "flate";
  # renovate: datasource=github-releases depName=home-operations/flate
  version = "0.7.3";

  src = pkgs.fetchFromGitHub {
    owner = "home-operations";
    repo = "flate";
    tag = "v${finalAttrs.version}";
    hash = "sha256-sm22CX+3B7roECyNz5v9yLaqlTPZN7GWMcNthBn5IpY=";
  };

  doCheck = false;

  vendorHash = "sha256-TyYkEU+UbN2B+rj0D3QxwEOGpwLKwHW9jZT/kitPYn8=";

  ldflags = [
    "-s"
    "-w"
  ];

  nativeBuildInputs = [ pkgs.installShellFiles ];

  postInstall = lib.optionalString (pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform) ''
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
