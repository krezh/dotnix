{
  lib,
  pkgs,
}:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "sofka";
  # renovate: datasource=github-releases depName=nklmilojevic/sofka
  version = "0.36.0";

  src = pkgs.fetchFromGitHub {
    owner = "nklmilojevic";
    repo = "sofka";
    tag = "v${version}";
    hash = "sha256-mxuJfZDP2ripRCADEB/qRecICbe58ZjEju0iqAoKiLA=";
  };

  cargoHash = "sha256-JSL9wou9Owa7fR7+78Hqxr5hE/hHjeZmqiJXTCUJbno=";

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
