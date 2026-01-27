{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:
buildGoModule rec {
  pname = "kubectl-rook-ceph";
  # renovate: datasource=github-releases depName=rook/kubectl-rook-ceph
  version = "0.9.5";

  src = fetchFromGitHub {
    owner = "rook";
    repo = "kubectl-rook-ceph";
    rev = "v${version}";
    hash = "sha256-OYK86GamU4m9vJUINfRbpM5U6mbjI3P6aiUp3+RZvIA=";
  };

  go = go-bin.latestStable;
  vendorHash = "sha256-D2WbLc6/FVm9YB7meWdJ5Of0WYBB+kKC2+AepdgwJAA=";

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
