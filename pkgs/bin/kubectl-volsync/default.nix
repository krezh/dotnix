{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-volsync";
  # renovate: datasource=github-releases depName=backube/volsync
  version = "0.10.0";

  src = fetchFromGitHub {
    owner = "backube";
    repo = "volsync";
    rev = "v${version}";
    hash = "sha256-+02rPf7YQQC9d/QRQtSPyER3MfeutpEkUUcFWnRdBsk=";
  };

  vendorHash = "sha256-kssvCsQYVWiOC5SFN3unqjR8dqrd6bRBuHXCOGQQi/4=";

  ldflags = [
    "-s"
    "-w"
  ];

  subPackages = [ "kubectl-volsync" ];

  meta = {
    description = "Asynchronous data replication for Kubernetes volumes";
    homepage = "https://github.com/backube/volsync";
    changelog = "https://github.com/backube/volsync/blob/${src.rev}/CHANGELOG.md";
    license = lib.licenses.agpl3Only;
    maintainers = [ ];
    mainProgram = "kubectl-volsync";
  };
}
