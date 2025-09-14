{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubectl-df-pv";
  # renovate: datasource=github-tag depName=yashbhutwala/kubectl-df-pv
  version = "0.3.0";

  src = fetchFromGitHub {
    owner = "yashbhutwala";
    repo = "kubectl-df-pv";
    rev = "v${version}";
    hash = "sha256-FxKqkxLMNfCXuahKTMod6kWKZ/ucYeIEFcS8BmpbLWg=";
  };

  vendorHash = "sha256-YkDPgN7jBvYveiyU8N+3Ia52SEmlzC0TGBQjUuIAaw0=";

  postInstall = ''
    mv $out/bin/df-pv $out/bin/kubectl-df-pv
  '';

  meta = {
    description = "df (disk free)-like utility for persistent volumes on kubernetes";
    homepage = "https://github.com/yashbhutwala/kubectl-df-pv";
    changelog = "https://github.com/yashbhutwala/kubectl-df-pv/releases/tag/v${version}";
    license = lib.licenses.asl20;
    maintainers = [ ];
    mainProgram = "df-pv";
  };
}
