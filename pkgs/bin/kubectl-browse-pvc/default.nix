{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:
(buildGoModule.override { go = go-bin.latestStable; }) rec {
  pname = "kubectl-browse-pvc";
  # renovate: datasource=github-releases depName=clbx/kubectl-browse-pvc
  version = "1.4.0";

  src = fetchFromGitHub {
    owner = "clbx";
    repo = "kubectl-browse-pvc";
    rev = "v${version}";
    hash = "sha256-dw7q28qNS3RcUuPZcDc4JqPNU1JTn1RJMLJ6KSkpjrU=";
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
