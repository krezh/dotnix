{ lib, buildGoModule, fetchFromGitHub, installShellFiles }:

buildGoModule rec {
  pname = "k9s";
  # renovate: datasource=github-tags depName=derailed/k9s
  version = "0.31.8";

  src = fetchFromGitHub {
    owner = "derailed";
    repo = "k9s";
    rev = "v${version}";
    hash = "sha256-sZtMeFoi3UJO5uV4zOez1TbpBCtfclGhZTrYGZ/+Mio=";
  };

  vendorHash = "sha256-ldNM9KpBVTLTEv1rJs1kNUtVn5qH2yvAqX/X++bIjGY=";

  ldflags = [
    "-s"
    "-w"
    "-X github.com/derailed/k9s/cmd.version=${version}"
    "-X github.com/derailed/k9s/cmd.commit=${src.rev}"
    "-X github.com/derailed/k9s/cmd.date=1970-01-01T00:00:00Z"
  ];

  GOWORK = "off";

  nativeBuildInputs = [ installShellFiles ];
  postInstall = ''
    installShellCompletion --cmd k9s \
      --bash <($out/bin/k9s completion bash) \
      --fish <($out/bin/k9s completion fish) \
      --zsh <($out/bin/k9s completion zsh)
  '';

  doCheck = false; # no tests
}
