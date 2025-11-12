{
  lib,
  rustPlatform,
  pkgs,
  makeWrapper,
  llvmPackages,
  ...
}:

rustPlatform.buildRustPackage {
  pname = "gulp";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
    makeWrapper
    llvmPackages.clang
  ];

  buildInputs = with pkgs; [
    wayland
    wayland-protocols
    wayland-scanner
    cairo
    pango
    libxkbcommon
    tesseract
    leptonica
    llvmPackages.libclang.lib
  ];

  env = {
    LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${llvmPackages.libclang.lib}/lib/clang/${llvmPackages.libclang.version}/include";
  };

  postInstall = ''
    wrapProgram $out/bin/gulp \
      --prefix PATH : ${lib.makeBinPath [ pkgs.tesseract ]}
  '';

  meta = {
    description = "A playful, compositor-agnostic Wayland screen selection tool with OCR support";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "gulp";
  };
}
