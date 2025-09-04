{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:

rustPlatform.buildRustPackage rec {
  pname = "hypr-showkey";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "cubismod";
    repo = "hypr-showkey";
    rev = "v${version}";
    hash = "sha256-mZL7ta6RUDTlbx7LPUtVoxP3FxmAxetZ+RnQLrIjzVE=";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  postPatch = ''
    ln -s ${./Cargo.lock} Cargo.lock
  '';

  meta = {
    description = "Parses your hyprland keybindings configs and displays as a fuzzy searchable TUI";
    homepage = "https://github.com/cubismod/hypr-showkey";
    license = lib.licenses.asl20;
    mainProgram = "hypr-showkey";
  };
}
