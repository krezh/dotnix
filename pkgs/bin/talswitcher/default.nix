{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  installShellFiles,
}:
buildGoModule rec {
  pname = "talswitcher";
  # renovate: datasource=github-releases depName=mirceanton/talswitcher
  version = "2.2.11";

  src = fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${version}";
    hash = "sha256-bn2SSXlZlfvCMan0oy/dHpMp5VxwLuEKBmWNhQlneEc=";
  };

  go = go-bin.latestStable;
  vendorHash = "sha256-tKc2T2DIXafLtLswSA2e/bdjkQW/2m7eW8Rt+aCJ/X4=";

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
