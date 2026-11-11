{
  lib,
  pkgs,
}:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "sofka";
  # renovate: datasource=github-releases depName=nklmilojevic/sofka
  version = "0.35.1";

  src = pkgs.fetchFromGitHub {
    owner = "nklmilojevic";
    repo = "sofka";
    tag = "v${version}";
    hash = "sha256-o9F54joKQ3ypZDs1Lw/AgGN42+fFJOJ+BE4Pou3KkXY=";
  };

  cargoHash = "sha256-l+nQJFB/ZMMO3AI/YoHvk2Lz4D1Q66o6SQJb/A2vdjs=";

  nativeBuildInputs = [ pkgs.pkg-config ];

  buildInputs = [
    pkgs.aws-lc
    pkgs.zstd
  ];

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
