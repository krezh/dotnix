{
  lib,
  stdenv,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  installShellFiles,
  versionCheckHook,
}:

buildGoModule rec {
  pname = "talosctl";
  # renovate: datasource=github-releases depName=siderolabs/talos
  version = "1.12.1";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${version}";
    hash = "sha256-N/kqQTlxpwI1vrCjhqtFe0YdyfxTzxIzoMsmPKwtaCM=";
  };

  go = go-bin.latestStable;
  vendorHash = "";

  ldflags = [
    "-s"
    "-w"
  ];

  env.GOWORK = "off";

  subPackages = [ "cmd/talosctl" ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd talosctl \
      --bash <($out/bin/talosctl completion bash) \
      --fish <($out/bin/talosctl completion fish) \
      --zsh <($out/bin/talosctl completion zsh)
  '';

  doCheck = false; # no tests

  doInstallCheck = true;
  nativeInstallCheckInputs = [ versionCheckHook ];
  versionCheckProgramArg = "version";

  meta = with lib; {
    description = "A CLI for out-of-band management of Kubernetes nodes created by Talos";
    homepage = "https://www.talos.dev/";
    license = licenses.mpl20;
    maintainers = [ ];
    mainProgram = pname;
  };
}
