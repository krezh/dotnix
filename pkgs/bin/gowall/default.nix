{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "gowall";
  # renovate: datasource=github-release depName=Achno/gowall
  version = "0.1.5";

  src = fetchFromGitHub {
    owner = "Achno";
    repo = "gowall";
    rev = "v${version}";
    hash = "sha256-4h7vRu1aqCGccKx2UiLSFNloqf22QUml4BHkKzzdwYA=";
  };

  vendorHash = "sha256-jNx4ehew+IBx7M6ey/rT0vb53+9OBVYSEDJv8JWfZIw=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = with lib; {
    description = "A tool to convert a Wallpaper's color scheme / palette";
    homepage = "https://github.com/Achno/gowall";
    license = licenses.mit;
    mainProgram = "gowall";
  };
}
