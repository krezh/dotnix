{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:
rustPlatform.buildRustPackage rec {
  pname = "sofka";
  # renovate: datasource=github-releases depName=nklmilojevic/sofka
  version = "0.31.10";

  src = fetchFromGitHub {
    owner = "nklmilojevic";
    repo = "sofka";
    tag = "v${version}";
    hash = "sha256-KVNgehQei1mWd13YgGu/nHrfnjAuVXwpM5P/Mp4Jmv0=";
  };

  cargoHash = "sha256-RIqkxdgpVaDL6tNgf2u4k/daWKoOP9tDE9f8bqCoXQ0=";

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
