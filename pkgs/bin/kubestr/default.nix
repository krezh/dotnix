{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "kubestr";
  # renovate: datasource=github-releases depName=kastenhq/kubestr
  version = "0.4.49";

  src = fetchFromGitHub {
    owner = "kastenhq";
    repo = "kubestr";
    rev = "v${version}";
    hash = "sha256-paBewecIv3LiSSwaLZKHXcT7jOjIcgIURcJEcz1KNtE=";
  };

  vendorHash = "sha256-A/id2ut4CHNa1Q59Az0VuyC/PbF2jsa1sMdDsuRemKM=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "";
    homepage = "https://github.com/kastenhq/kubestr";
    license = lib.licenses.asl20;
    maintainers = [ ];
    mainProgram = pname;
  };
}
