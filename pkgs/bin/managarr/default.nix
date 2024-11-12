{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  openssl,
  stdenv,
  darwin,
}:

rustPlatform.buildRustPackage rec {
  pname = "managarr";
  # renovate: datasource=github-releases depName=Dark-Alex-17/managarr
  version = "0.2.2";

  src = fetchFromGitHub {
    owner = "Dark-Alex-17";
    repo = "managarr";
    rev = "v${version}";
    hash = "sha256-TAt1mFOyk4qVrwYVj2Irjx8NTtEFe3VASmbE0QxbIwA=";
  };

  cargoHash = "sha256-bgf9aNyPOwSo2VIwg6gogTxOOGdiIa5v3BNXYg1MTUI=";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs =
    [
      openssl
    ]
    ++ lib.optionals stdenv.isDarwin [
      darwin.apple_sdk.frameworks.Security
    ];

  meta = {
    description = "A TUI for managing *arr servers. Built with 🤎 in Rust";
    homepage = "https://github.com/Dark-Alex-17/managarr";
    changelog = "https://github.com/Dark-Alex-17/managarr/blob/${src.rev}/CHANGELOG.md";
    license = lib.licenses.mit;
    maintainers = [ ];
    mainProgram = "managarr";
  };
}
