{
  lib,
  buildGo124Module,
  fetchFromGitHub,
  installShellFiles,
}:
buildGo124Module rec {
  pname = "talswitcher";
  # renovate: datasource=github-releases depName=mirceanton/talswitcher
  version = "2.1.8";

  src = fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${version}";
    hash = "sha256-KaqRqJN5s1DHa+g8a3gRYhXnznuVy4sohC/we62bBSA=";
  };

  vendorHash = "sha256-UtW09qNt2Fa3wilnLZ8XMK/Vxpk5a+OvbVY3RkrDo+I=";

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
