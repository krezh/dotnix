{
  lib,
  pkgs,
}:
(pkgs.buildGoModule.override { go = pkgs.go-bin.latestStable; }) (finalAttrs: {
  pname = "talswitcher";
  # renovate: datasource=github-releases depName=mirceanton/talswitcher
  version = "2.2.45";

  src = pkgs.fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${finalAttrs.version}";
    hash = "sha256-+Ek5audnfcxi/mUsDZeuBkls9X5bi4w4+m0mavtczEY=";
  };

  vendorHash = "sha256-UzcUhFDDKCGrQyI8B5+4W6UR/z5GU2uKeZZ7s/GKU4g=";

  preBuild = ''
    export HOME="$TMPDIR"
    mkdir -p "$HOME/.talos/configs"
  '';

  ldflags = [
    "-s"
    "-w"
    "-X=github.com/mirceanton/${finalAttrs.pname}/cmd.version=${finalAttrs.version}"
  ];

  nativeBuildInputs = [ pkgs.installShellFiles ];

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
