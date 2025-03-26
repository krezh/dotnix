{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-volsync";
  # renovate: datasource=github-releases depName=backube/volsync
  version = "0.12.1";

  src = fetchFromGitHub {
    owner = "backube";
    repo = "volsync";
    rev = "v${version}";
    hash = "sha256-8aqZakHtqFII+7NxAFjQuaJtAAhrZubEvJIQe5COqJ8=";
  };

  vendorHash = "sha256-5IDIaqJKsWYqA9KHGZ5lbzt3UaEACkVoWoj8HDBhc5g=";

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
