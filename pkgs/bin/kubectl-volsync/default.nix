{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-volsync";
  # renovate: datasource=github-releases depName=backube/volsync
  version = "0.11.0";

  src = fetchFromGitHub {
    owner = "backube";
    repo = "volsync";
    rev = "v${version}";
    hash = "sha256-14TG5wEFHpkMkiiacbPDY1IyXafrFBLHKZ9chkVBiF8=";
  };

  vendorHash = "sha256-6IwkCeBsKV6Ig9MY16hO/YSAKMi9TdKA4luSerVXK88=";

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
