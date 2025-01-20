{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubestr";
  # renovate: datasource=github-releases depName=kastenhq/kubestr
  version = "0.4.48";

  src = fetchFromGitHub {
    owner = "kastenhq";
    repo = "kubestr";
    rev = "v${version}";
    hash = "sha256-mSqiz6rK88JJ7wXw4RIqx5EvKIPCNnZerFhtDuysyWQ=";
  };

  vendorHash = "sha256-zD7uTx6vobjdjGAU7LLxmRgEaQIpVLKw3SGL2wbptYg=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "";
    homepage = "https://github.com/kastenhq/kubestr";
    license = lib.licenses.asl20;
    maintainers = [ ];
    mainProgram = "kubestr";
  };
}
