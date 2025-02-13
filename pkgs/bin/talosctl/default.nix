{
  lib,
  buildGoModule,
  fetchFromGitHub,
  installShellFiles,
}:
buildGoModule rec {
  pname = "talosctl";
  # renovate: datasource=github-releases depName=siderolabs/talos
  version = "1.9.4";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${version}";
    hash = "sha256-gD9fIBILqSFExTIziNx6Hy6TpsBC90yHSfSDB9fhKwA=";
  };

  vendorHash = "sha256-ECTR1U2RlatLK3c3UP/aHzYMkN6WxPsHuwoXUzIYsAA=";

  ldflags = [
    "-s"
    "-w"
  ];

  # This is needed to deal with workspace issues during the build
  overrideModAttrs = _: { GOWORK = "off"; };
  GOWORK = "off";

  subPackages = [ "cmd/talosctl" ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd talosctl \
      --bash <($out/bin/talosctl completion bash) \
      --fish <($out/bin/talosctl completion fish) \
      --zsh <($out/bin/talosctl completion zsh)
  '';

  doCheck = false; # no tests

  meta = with lib; {
    description = "A CLI for out-of-band management of Kubernetes nodes created by Talos";
    homepage = "https://www.talos.dev/";
    license = licenses.mpl20;
    maintainers = [ ];
  };
}
