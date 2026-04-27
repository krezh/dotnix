{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  installShellFiles,
}:
(buildGoModule.override { go = go-bin.latestStable; }) rec {
  pname = "talswitcher";
  # renovate: datasource=github-releases depName=mirceanton/talswitcher
  version = "2.2.21";

  src = fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${version}";
    hash = "sha256-pa4PrNQkrCM6clk8n83SJieDqqTC+uyavChDsEEen8g=";
  };

  vendorHash = "sha256-MpGwlyhBC3AuaP0zr/JDdeqn4DRdxzENMgSlmX1RdLs=";

  # Make build write to a writable tempdir instead of /homeless-shelter
  preBuild = ''
    export HOME="$TMPDIR"
    mkdir -p "$HOME/.talos/configs"
  '';

  ldflags = [
    "-s"
    "-w"
    "-X=github.com/mirceanton/${pname}/cmd.version=${version}"
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
    mainProgram = pname;
  };
}
