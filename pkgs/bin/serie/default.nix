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
  version = "0.7.0";

  src = fetchFromGitHub {
    owner = "lusingander";
    repo = "serie";
    tag = "v${finalAttrs.version}";
    hash = "sha256-J84xop9QGRa9pgHGF8ioLwmnXu1t5iO9ZLV2MoYRdEI=";
  };

  cargoHash = "sha256-B9Fn4okfS/OwhR34YwyjhPvpK6DLFuVY0BRubj4Y4MA=";

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
