{
  buildGoModule,
  fetchFromGitHub,
  go-bin,
  lib,
  installShellFiles,
  stdenv,
}:
(buildGoModule.override { go = go-bin.latestStable; }) (finalAttrs: {
  pname = "zli";
  # renovate: datasource=github-releases depName=project-zot/zot
  version = "2.1.18";

  src = fetchFromGitHub {
    owner = "project-zot";
    repo = "zot";
    rev = "v${finalAttrs.version}";
    hash = "sha256-cYH+4nvrdtKRYx3bap8Ndhu8MtwOWQREnmCa7NE/+O8=";
  };

  vendorHash = "sha256-iB/hk4W0yuuptjTso0GINrMJ00XTyCF3EfcsUg5BoPE=";

  subPackages = [ "cmd/zli" ];

  tags = [ "search" ];

  ldflags = [
    "-s"
    "-w"
  ];

  nativeBuildInputs = [ installShellFiles ];

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd ${finalAttrs.pname} \
      --bash <($out/bin/${finalAttrs.pname} completion bash) \
      --fish <($out/bin/${finalAttrs.pname} completion fish) \
      --zsh <($out/bin/${finalAttrs.pname} completion zsh)
  '';

  doCheck = false;

  meta = {
    description = "CLI client for the Zot OCI container registry";
    homepage = "https://zotregistry.dev";
    license = lib.licenses.asl20;
    mainProgram = "zli";
  };
})
