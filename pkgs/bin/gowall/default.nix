{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "gowall";
  version = "0.1.4";

  src = fetchFromGitHub {
    owner = "Achno";
    repo = "gowall";
    rev = "v${version}";
    hash = "sha256-6Fyanvqqf87liJFpYZq/7pG9bZjYESJH5K5In27ZGgk=";
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
