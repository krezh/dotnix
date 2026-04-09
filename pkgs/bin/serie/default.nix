{
  lib,
  craneLib,
  fetchFromGitHub,
  pkg-config,
  stdenv,
  wayland,
}:
craneLib.buildPackage rec {
  pname = "serie";
  # renovate: datasource=github-releases depName=lusingander/serie
  version = "0.7.1";

  src = fetchFromGitHub {
    owner = "lusingander";
    repo = "serie";
    tag = "v${version}";
    hash = "sha256-tNMNbxPuWNXfBdQglq6PekJV93AdhO+zqAA+dyNqdcQ=";
  };

  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src pname strictDeps;
    nativeBuildInputs = [ pkg-config ];
    buildInputs = lib.optionals stdenv.isLinux [ wayland ];
  };

  nativeBuildInputs = [ pkg-config ];
  buildInputs = lib.optionals stdenv.isLinux [ wayland ];

  doCheck = false;

  meta = {
    description = "A rich git commit graph in your terminal, like magic";
    homepage = "https://github.com/lusingander/serie";
    license = lib.licenses.mit;
    mainProgram = pname;
  };
}
