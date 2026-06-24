{
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  lib,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "zot";
  # renovate: datasource=github-releases depName=project-zot/zot
  version = "2.1.18";

  src = fetchFromGitHub {
    owner = "project-zot";
    repo = "zot";
    rev = "v${finalAttrs.version}";
    hash = "sha256-cYH+4nvrdtKRYx3bap8Ndhu8MtwOWQREnmCa7NE/+O8=";
  };

  vendorHash = "sha256-iB/hk4W0yuuptjTso0GINrMJ00XTyCF3EfcsUg5BoPE=";

  subPackages = [ "cmd/zot" ];

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false;

  meta = {
    description = "Production-ready vendor-neutral OCI-native container image registry";
    homepage = "https://zotregistry.dev";
    license = lib.licenses.asl20;
    mainProgram = "zot";
  };
})
