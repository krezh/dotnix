{
  lib,
  buildGoModule,
}:

buildGoModule {
  pname = "klim";
  version = "0.1.0";

  src = builtins.path {
    path = ./src;
    name = "klim-src";
  };

  vendorHash = "sha256-A9CEDNjPwNWaKv0ASKYdTPDZ+APv+dWn6dCY/EjLpT8=";

  ldflags = [
    "-s"
    "-w"
    "-X main.version=0.1.0"
  ];

  meta = with lib; {
    description = "Kubernetes Resource Recommender";
    homepage = "https://github.com/krezh/klim";
    license = licenses.mit;
    mainProgram = "klim";
  };
}
