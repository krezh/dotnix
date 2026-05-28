{
  lib,
  rustPlatform,
  fetchFromForgejo,
  pkg-config,
  openssl,
  sqlite,
}:
rustPlatform.buildRustPackage rec {
  pname = "towonel";
  # renovate: datasource=forgejo-releases registryUrl=https://codeberg.org depName=towonel/towonel
  version = "0.1.10";

  src = fetchFromForgejo {
    domain = "codeberg.org";
    owner = "towonel";
    repo = "towonel";
    tag = "v${version}";
    hash = "sha256-jwN4ro/B7a5Kgz1WJXmCPPzCDuSRBUWcUbLtZKM7pYM=";
  };

  cargoHash = "sha256-vqPXGDFT+8wRX+RpJMr8OYFfikNbVLmIb3SMIhVEvXU=";

  cargoBuildFlags = [
    "--package"
    "towonel-node"
  ];
  doCheck = false;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    openssl
    sqlite
  ];

  env = {
    LIBSQLITE3_SYS_USE_PKG_CONFIG = true;
    OPENSSL_NO_VENDOR = true;
  };

  meta = {
    description = "Exposes HTTP(S) services behind NAT, CGNAT, or dynamic IPs without opening inbound ports";
    homepage = "https://codeberg.org/towonel/towonel";
    changelog = "https://codeberg.org/towonel/towonel/releases/tag/v${version}";
    license = lib.licenses.mit;
    mainProgram = "towonel";
  };
}
