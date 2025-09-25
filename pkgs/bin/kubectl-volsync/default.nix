{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-volsync";
  # renovate: datasource=github-releases depName=backube/volsync
  version = "0.13.1";

  src = fetchFromGitHub {
    owner = "backube";
    repo = "volsync";
    rev = "v${version}";
    hash = "sha256-7biUwChD4mauaSszeaVgWgVJp6kAJbab9DcKF/OfM5Q=";
  };

  vendorHash = "sha256-okhKVf+cz/KGV5iKGf/JtFlNzKYJlMwqCa9iGiTtFv4=";

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
    mainProgram = pname;
  };
}
