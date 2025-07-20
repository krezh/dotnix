{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:

rustPlatform.buildRustPackage rec {
  pname = "yaml2nix";
  version = "0.3.0";

  src = fetchFromGitHub {
    owner = "euank";
    repo = "yaml2nix";
    rev = "v${version}";
    hash = "sha256-DkHWWpvBco2yodyOk40LjTNcoaJ1bFKf0JY9OwWgy5M=";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "serde-nix-0.1.0" = "sha256-flyOG50FasL5PdnBRNAJSHkb6GGkC8DvXYk1s/jUmps=";
    };
  };

  meta = {
    description = "";
    homepage = "https://github.com/euank/yaml2nix";
    license = lib.licenses.gpl3Only;
    maintainers = [ ];
    mainProgram = pname;
  };
}
