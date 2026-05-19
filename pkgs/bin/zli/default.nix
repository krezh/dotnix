{
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  lib,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "zli";
  # renovate: datasource=github-releases depName=project-zot/zot
  version = "2.1.16";

  src = fetchFromGitHub {
    owner = "project-zot";
    repo = "zot";
    rev = "v${finalAttrs.version}";
    hash = "sha256-eEyaV0PZjqYIWOgof6xejK+6TK5Ubat778ovjGQzNzk=";
  };

  vendorHash = "sha256-IWghiDENDoKPd6zvTpOnA5x2lTs/N6dI+7hAHZthds8=";

  subPackages = [ "cmd/zli" ];

  tags = [ "search" ];

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false;

  meta = {
    description = "CLI client for the Zot OCI container registry";
    homepage = "https://zotregistry.dev";
    license = lib.licenses.asl20;
    mainProgram = "zli";
  };
})
