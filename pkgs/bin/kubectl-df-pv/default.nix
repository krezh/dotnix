{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go-bin,
}:
buildGoModule rec {
  pname = "kubectl-df-pv";
  # renovate: datasource=github-releases depName=yashbhutwala/kubectl-df-pv
  version = "0.4.1";

  src = fetchFromGitHub {
    owner = "yashbhutwala";
    repo = "kubectl-df-pv";
    rev = "v${version}";
    hash = "sha256-dGWGPamVD/26iEgKQcWGKpFIMMlDivFpD/XzmjCr8pQ=";
  };

  go = go-bin.latestStable;
  vendorHash = "sha256-J15tCwYiVSPa2hSB3DMFtVW9Uer7pFMCD1OpCobnYMc=";

  postInstall = ''
    mv $out/bin/df-pv $out/bin/kubectl-df-pv
  '';

  meta = {
    description = "df (disk free)-like utility for persistent volumes on kubernetes";
    homepage = "https://github.com/yashbhutwala/kubectl-df-pv";
    changelog = "https://github.com/yashbhutwala/kubectl-df-pv/releases/tag/v${version}";
    license = lib.licenses.asl20;
    maintainers = [];
    mainProgram = "df-pv";
  };
}
