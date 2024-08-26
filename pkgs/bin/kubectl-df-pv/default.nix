{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-df-pv";
  version = "0.3.0";

  src = fetchFromGitHub {
    owner = "yashbhutwala";
    repo = "kubectl-df-pv";
    rev = "v${version}";
    hash = "sha256-FxKqkxLMNfCXuahKTMod6kWKZ/ucYeIEFcS8BmpbLWg=";
  };

  vendorHash = "sha256-YkDPgN7jBvYveiyU8N+3Ia52SEmlzC0TGBQjUuIAaw0=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "Kubectl plugin - giving admins df (disk free) like utility for persistent volumes";
    homepage = "https://github.com/yashbhutwala/kubectl-df-pv";
    license = lib.licenses.asl20;
    maintainers = [ ];
    mainProgram = "df-pv";
  };
}
