{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  stdenv,
  wayland,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "serie";
  # renovate: datasource=github-releases depName=lusingander/serie
  version = "0.7.1";

  src = fetchFromGitHub {
    owner = "lusingander";
    repo = "serie";
    tag = "v${finalAttrs.version}";
    hash = "sha256-tNMNbxPuWNXfBdQglq6PekJV93AdhO+zqAA+dyNqdcQ=";
  };

  cargoHash = "sha256-UWrnhczknl/F5gSA9S4W+Ub5zzB7XuQ358d7XVXRjjQ=";

  doCheck = false;

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = lib.optionals stdenv.isLinux [
    wayland
  ];

  meta = {
    description = "A rich git commit graph in your terminal, like magic";
    homepage = "https://github.com/lusingander/serie";
    license = lib.licenses.mit;
    mainProgram = "serie";
  };
})
