{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:
rustPlatform.buildRustPackage rec {
  pname = "sofka";
  # renovate: datasource=github-releases depName=nklmilojevic/sofka
  version = "0.27.3";

  src = fetchFromGitHub {
    owner = "nklmilojevic";
    repo = "sofka";
    tag = "v${version}";
    hash = "sha256-OnfZKj/HK8qs6HBFh1e4ExdCVOE3QxDgqgVV1nARSYo=";
  };

  cargoHash = "sha256-sO8/DgMGe7649AP3emdect1pIrn3YMnGPioK4MjqF+c=";

  doCheck = false;

  meta = {
    description = "Kubernetes TUI, reimagined in Rust - built on kube-rs and ratatui, async-first from the ground up";
    homepage = "https://github.com/nklmilojevic/sofka";
    license = with lib.licenses; [
      mit
      asl20
    ];
    mainProgram = "sofka";
  };
}
