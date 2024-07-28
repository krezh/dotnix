{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "gowall";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "Achno";
    repo = "gowall";
    rev = "v${version}";
    hash = "sha256-4Xs9sEt4+6YWJz6QKo/ih6MXX5b9eGI9fdw5O7u68Bo=";
  };

  vendorHash = "sha256-IbUOhqg3lKqV5h9MZagsnmqmCLn/gSopxWSyxHp7AJ8=";

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
