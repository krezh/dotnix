{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "up";
  # renovate: datasource=github-releases depName=jesusprubio/up
  version = "1.1.0";

  src = fetchFromGitHub {
    owner = "jesusprubio";
    repo = "up";
    rev = "v${version}";
    hash = "sha256-sr57l/yDCsc+PUCjtSrOKdtAxqObDJpO6zF6eWVKezc=";
  };

  vendorHash = "sha256-/Gsqc8rEptMBItqeb/N/gE4V3iUGZa8k1GqUR1+togY=";

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false; # no tests

  meta = {
    description = "Troubleshoot problems with your Internet connection";
    homepage = "https://github.com/jesusprubio/up";
    license = lib.licenses.mit;
    maintainers = [ ];
    mainProgram = "up";
  };
}
