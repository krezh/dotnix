{
  lib,
  buildGoModule,
  fetchFromGitHub,
  installShellFiles,
}:

buildGoModule rec {
  pname = "gowall";
  # renovate: datasource=github-releases depName=Achno/gowall
  version = "0.2.3";

  src = fetchFromGitHub {
    owner = "Achno";
    repo = "gowall";
    rev = "v${version}";
    hash = "sha256-HZEVH3T4dmBE4OMPjtHj3qdeT4i27+YhZWJgYqbg5ss=";
  };

  vendorHash = "sha256-zQoXrQnejng1jBKRMaQzQaZYKWxJPXjgdplnuVhazuM=";

  # Required to workaround test error:
  #   panic: mkdir /homeless-shelter: permission denied
  HOME = "$TMPDIR";

  ldflags = [
    "-s"
    "-w"
  ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd gowall \
      --bash <($out/bin/gowall completion bash) \
      --fish <($out/bin/gowall completion fish) \
      --zsh <($out/bin/gowall completion zsh)
  '';

  meta = with lib; {
    description = "A tool to convert a Wallpaper's color scheme / palette";
    homepage = "https://github.com/Achno/gowall";
    license = licenses.mit;
    mainProgram = pname;
  };
}
