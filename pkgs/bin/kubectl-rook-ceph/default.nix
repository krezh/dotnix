{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-rook-ceph";
  # renovate: datasource=github-tag depName=rook/kubectl-rook-ceph
  version = "0.9.3";

  src = fetchFromGitHub {
    owner = "rook";
    repo = "kubectl-rook-ceph";
    rev = "v${version}";
    hash = "sha256-stWuRej3ogGETLzVabMRfakoK358lJbK56/hjBh2k2M=";
  };

  vendorHash = "sha256-fB3S946nv1uH9blek6w2EmmYYcdnBcEbmYELfPH9A04=";

  ldflags = [
    "-s"
    "-w"
  ];

  postInstall = ''
    mv $out/bin/cmd $out/bin/kubectl-rook
  '';

  meta = {
    description = "Krew plugin to run kubectl commands with rook-ceph";
    homepage = "https://github.com/rook/kubectl-rook-ceph";
    changelog = "https://github.com/rook/kubectl-rook-ceph/releases/tag/v${version}";
    license = lib.licenses.asl20;
    maintainers = [ ];
    mainProgram = "kubectl-rook";
  };
}
