{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:
(buildGoModule.override { go = go-bin.latestStable; }) rec {
  pname = "kubectl-rook-ceph";
  # renovate: datasource=github-releases depName=rook/kubectl-rook-ceph
  version = "0.9.6";

  src = fetchFromGitHub {
    owner = "rook";
    repo = "kubectl-rook-ceph";
    rev = "v${version}";
    hash = "sha256-9keMfpYRtJ49fg8rVtwImPlkVHYKwjKWMtLHxhzUE18=";
  };

  vendorHash = "sha256-rXSXxSUR/F1eSsGUe1penn9eM2qTcPgE5pZmafQSISU=";

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
