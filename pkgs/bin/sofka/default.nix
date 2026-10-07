{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:
rustPlatform.buildRustPackage rec {
  pname = "sofka";
  # renovate: datasource=github-releases depName=nklmilojevic/sofka
  version = "0.31.1";

  src = fetchFromGitHub {
    owner = "nklmilojevic";
    repo = "sofka";
    tag = "v${version}";
    hash = "sha256-779LE4ttADC12QImV37W8L5cJSlcUwV5SuMeToD2ARk=";
  };

  cargoHash = "sha256-gfxbzJvTZKOK17BQEmEb7oRv9SyLfToHbSkE74jID3Q=";

  doCheck = false;

  meta = {
    description = "Kubernetes TUI, reimagined in Rust - built on kube-rs and ratatui, async-first from the ground up";
    homepage = "https://github.com/nklmilojevic/sofka";
    changelog = "https://github.com/nklmilojevic/sofka/releases/tag/v${version}";
    license = with lib.licenses; [
      mit
      asl20
    ];
    mainProgram = "sofka";
  };
}
