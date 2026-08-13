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
  version = "2.2.33";

  src = fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${finalAttrs.version}";
    hash = "sha256-/G/W+G83IKgVXQqXt7mcu+iw9faDpEcQUjVwMCh6Bxk=";
  };

  vendorHash = "sha256-6rAc/q/a4meDNwnOSMKvKKJJHUs8r2gqlkmZtM1qtl0=";

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
    license = lib.licenses.mit;
    mainProgram = finalAttrs.pname;
  };
})
