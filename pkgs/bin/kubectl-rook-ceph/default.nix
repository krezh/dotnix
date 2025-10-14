{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-rook-ceph";
  # renovate: datasource=github-releases depName=rook/kubectl-rook-ceph
  version = "0.9.4";

  src = fetchFromGitHub {
    owner = "rook";
    repo = "kubectl-rook-ceph";
    rev = "v${version}";
    hash = "sha256-t63m5cUIApAOBF1Nb8u2/Xkyi1OAGnaLSVWFyLec8AA=";
  };

  vendorHash = "sha256-8KrTfryEiTqF13NQ5xS1d9mIZI3ranA8+EkKUHu2mVE=";

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
