{
  buildGoModule,
  fetchFromGitHub,
  installShellFiles,
}:

buildGoModule rec {
  pname = "kubectl-login";
  # renovate: datasource=github-release depName=ghcr.io/TremoloSecurity/kubectl-login
  version = "0.0.8";

  src = fetchFromGitHub {
    owner = "TremoloSecurity";
    repo = "kubectl-login";
    rev = "v${version}";
    hash = "sha256-0EG97KULCVn29TZVWXqnbGNvDkm5y1LAeiGlF/n/g60=";
  };

  vendorHash = "sha256-3f7J2VXhMJOJjxwy+UARW4GQbcdfI1FK0f2iHansTSc=";

  ldflags = [
    "-s"
    "-w"
  ];

  GOWORK = "off";

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd ouctl \
      --bash <($out/bin/kubectl-login completion bash) \
      --fish <($out/bin/kubectl-login completion fish) \
      --zsh <($out/bin/kubectl-login completion zsh)
  '';

  doCheck = false; # no tests
}
