{
  lib,
  buildGoModule,
  fetchFromGitHub,
  installShellFiles,
}:
buildGoModule rec {
  pname = "talosctl";
  # renovate: datasource=github-releases depName=siderolabs/talos
  version = "1.12.0";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${version}";
    hash = "sha256-u8/T01PWBGH3bJCNoC+FIzp8aH05ci4Kr3eHHWPDRkI=";
  };

  vendorHash = "sha256-LLtbdKq028EEs8lMt3uiwMo2KMJ6nJKf6xFyLJlg+oM=";

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
    mainProgram = pname;
  };
}
