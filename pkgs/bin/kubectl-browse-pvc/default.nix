{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "kubectl-browse-pvc";
  # renovate: datasource=github-releases depName=clbx/kubectl-browse-pvc
  version = "1.4.4";

  src = fetchFromGitHub {
    owner = "clbx";
    repo = "kubectl-browse-pvc";
    rev = "v${finalAttrs.version}";
    hash = "sha256-xWNyZoYbyjnx61qpud91K2BpS3+pJ77ay1b3vF43aW4=";
  };

  ldflags = [
    "-s"
    "-w"
    "-X main.Version=${finalAttrs.version}"
  ];

  vendorHash = "sha256-cL/5nNOpo8MM1/0D+vomB60KUeH6/YP5j4DJepUx9iE=";

  meta = {
    description = "Kubectl plugin for browsing PVCs on the command line";
    homepage = "https://github.com/clbx/kubectl-browse-pvc";
    license = lib.licenses.mit;
    mainProgram = finalAttrs.pname;
    platforms = lib.platforms.all;
  };
})
