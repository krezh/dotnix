{ lib, buildGoModule, fetchFromGitHub, installShellFiles }:

buildGoModule rec {
  pname = "flux";
  version = "2.2.3";

  src = fetchFromGitHub {
    owner = "fluxcd";
    repo = "flux2";
    rev = "v${version}";
    hash = "sha256-1Z9EXqK+xnFGeWjoac1QZwOoMiYRRU1HEAZRaEpUOYs=";
  };

  vendorHash = "sha256-UPX5V3VwpX/eDy9ktqpvYb0JOzKRHH2nIQZzZ0jrYoQ=";

  ldflags = [ "-s" "-w" "-X" "main.VERSION=$(VERSION)" ];

  GOWORK = "off";

  subPackages = [ "cmd/flux" ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd flux \
      --bash <($out/bin/flux completion bash) \
      --fish <($out/bin/flux completion fish) \
      --zsh <($out/bin/flux completion zsh)
  '';

  doCheck = false; # no tests
}
