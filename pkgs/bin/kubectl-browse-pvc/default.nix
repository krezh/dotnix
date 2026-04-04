{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:
(buildGoModule.override { go = go-bin.latestStable; }) rec {
  pname = "kubectl-browse-pvc";
  # renovate: datasource=github-releases depName=clbx/kubectl-browse-pvc
  version = "1.4.2";

  src = fetchFromGitHub {
    owner = "clbx";
    repo = "kubectl-browse-pvc";
    rev = "v${version}";
    hash = "sha256-6tcRMRBfCuLib1paN1O73/so/n9yRobgy5fYd5ihTX8=";
  };

  ldflags = [
    "-s"
    "-w"
    "-X main.Version=${version}"
  ];

  vendorHash = "sha256-nrsIwjql/EBA1ch8DIk7QEiET3LcoOgtEW55LQgmaA4=";

  meta = {
    description = "Kubectl plugin for browsing PVCs on the command line";
    homepage = "https://github.com/clbx/kubectl-browse-pvc";
    license = lib.licenses.mit;
    maintainers = [ ];
    mainProgram = pname;
    platforms = lib.platforms.all;
  };
}
