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
  pname = "see";
  # renovate: datasource=github-releases depName=guilhermeprokisch/see
  version = "0.5.2";

  src = fetchFromGitHub {
    owner = "guilhermeprokisch";
    repo = "see";
    rev = "v${version}";
    hash = "sha256-cvWppG4TYmR/01W/VP+w3uzRDWWUJe7qBnEFlHBy+Gs=";
  };

  cargoHash = "sha256-cV+5JBuUXpaFnSIhoN6oQBDrIEqdq+Jg0lbA8vnPxwk=";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs =
    [
      openssl
    ]
    ++ lib.optionals stdenv.isDarwin [
      darwin.apple_sdk.frameworks.Security
      darwin.apple_sdk.frameworks.SystemConfiguration
    ];

  env = {
    OPENSSL_NO_VENDOR = true;
  };

  meta = {
    description = "A cute cat(1) for the terminal with advanced code viewing, Markdown rendering, 🌳  tree-sitter syntax highlighting, images view and more";
    homepage = "https://github.com/guilhermeprokisch/see";
    license = lib.licenses.mit;
    maintainers = [ ];
    mainProgram = "see";
  };
}
