{
  lib,
  stdenv,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  installShellFiles,
  versionCheckHook,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "talosctl";
  # renovate: datasource=github-releases depName=siderolabs/talos
  version = "v1.13.2";

  src = fetchFromGitHub {
    owner = "siderolabs";
    repo = "talos";
    rev = "v${finalAttrs.version}";
    hash = "sha256-Bhpy6+oO5oVKziR53Kg2/hO+Xg5Us3PHxkQjJ4j4t/I=";
  };

  vendorHash = "sha256-nsRehIMqfib3cQZ83peS0o7kAHsVHctiX0LYiOR2C3A=";

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

  doCheck = false;

  HOME = "$TMPDIR";

  doInstallCheck = true;
  nativeInstallCheckInputs = [ versionCheckHook ];
  versionCheckProgramArg = "version";

  meta = {
    description = "A CLI for out-of-band management of Kubernetes nodes created by Talos";
    homepage = "https://www.talos.dev/";
    license = lib.licenses.mpl20;
    mainProgram = finalAttrs.pname;
  };
})
