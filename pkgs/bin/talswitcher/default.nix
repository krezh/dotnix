{
  lib,
  buildGo124Module,
  fetchFromGitHub,
  installShellFiles,
}:
buildGo124Module rec {
  pname = "talswitcher";
  # renovate: datasource=github-releases depName=mirceanton/talswitcher
  version = "2.1.3";

  src = fetchFromGitHub {
    owner = "mirceanton";
    repo = "talswitcher";
    rev = "v${version}";
    hash = "sha256-1Sink1oxI7y8z60i4+gXIe25bUQWOqV/cU3YLIukRSk=";
  };

  vendorHash = "sha256-WaJjbNEMa0nxTGf/snFBPUbc9YrWMg/YCVBzbLEstJo=";

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
