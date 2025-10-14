{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-browse-pvc";
  # renovate: datasource=github-releases depName=clbx/kubectl-browse-pvc
  version = "1.3.0";

  src = fetchFromGitHub {
    owner = "clbx";
    repo = "kubectl-browse-pvc";
    rev = "v${version}";
    hash = "sha256-8O36JLNfrh+/9JqJjeeSEO88uYkoo6OXCraK385tGvM=";
  };

  sourceRoot = "${src.name}/src";

  ldflags = [
    "-s"
    "-w"
    "-X main.Version=${version}"
  ];

  vendorHash = "sha256-WwEFtiWP9rQnOjMNnY8nFBKvw0Gp29wcKrLrUqOr7DQ=";

  meta = {
    description = "Kubectl plugin for browsing PVCs on the command line";
    homepage = "https://github.com/clbx/kubectl-browse-pvc";
    license = lib.licenses.mit;
    maintainers = [ ];
    mainProgram = pname;
    platforms = lib.platforms.all;
  };
}
