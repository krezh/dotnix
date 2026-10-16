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
  version = "2.2.42";

  src = fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${finalAttrs.version}";
    hash = "sha256-dPqQcA/CQ+Z+scTV0X5YlF53cCYEhI3le7AxCTQ1Cnk=";
  };

  vendorHash = "sha256-DbIb3HhG8SWYH7zbRJlSwiODiHxzR6/jc1BYBUp9PXg=";

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
