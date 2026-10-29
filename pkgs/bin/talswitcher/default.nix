{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  installShellFiles,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "talswitcher";
  # renovate: datasource=github-releases depName=mirceanton/talswitcher
  version = "2.2.43";

  src = fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${finalAttrs.version}";
    hash = "sha256-coUxudbFTpVx4Ir+9SeTBxYtDSNGvFLS5VFRbhqeSQM=";
  };

  vendorHash = "sha256-qF02mIrymELdv9CaQw+dB7rx/zX9LkVD7rKWTQBMoS0=";

  preBuild = ''
    export HOME="$TMPDIR"
    mkdir -p "$HOME/.talos/configs"
  '';

  ldflags = [
    "-s"
    "-w"
    "-X=github.com/mirceanton/${finalAttrs.pname}/cmd.version=${finalAttrs.version}"
  ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd talswitcher \
      --bash <($out/bin/talswitcher completion bash) \
      --fish <($out/bin/talswitcher completion fish) \
      --zsh <($out/bin/talswitcher completion zsh)
  '';

  meta = {
    description = "A simple tool to help manage multiple talosconfig files";
    homepage = "https://github.com/mirceanton/talswitcher";
    changelog = "https://github.com/mirceanton/talswitcher/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.mit;
    mainProgram = finalAttrs.pname;
  };
})
