{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-volsync";
  # renovate: datasource=github-releases depName=backube/volsync
  version = "0.12.0";

  src = fetchFromGitHub {
    owner = "backube";
    repo = "volsync";
    rev = "v${version}";
    hash = "sha256-RE21laBF4SfoTKi5iN+f25UkNeomt+bCJFBDVrstYtk=";
  };

  vendorHash = "sha256-nndKpOSqFtmY60M7lOdId9mpR3OixKFw5FiUkrhJqv0=";

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
