{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:
rustPlatform.buildRustPackage rec {
  pname = "sofka";
  # renovate: datasource=github-releases depName=nklmilojevic/sofka
  version = "0.27.2";

  src = fetchFromGitHub {
    owner = "nklmilojevic";
    repo = "sofka";
    tag = "v${version}";
    hash = "sha256-BMevL6un7InPCrhWmyEgY4UlooHe6oVGL/jRu/FmCxo=";
  };

  cargoHash = "sha256-uvq++r4gbtEhlbTNEk+/7SLNvNrl+5LbDC4N5xWH4H4=";

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
