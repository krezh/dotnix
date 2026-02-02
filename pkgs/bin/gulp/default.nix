{
  lib,
  rustPlatform,
  pkgs,
  makeWrapper,
  llvmPackages,
  installShellFiles,
  ...
}:
rustPlatform.buildRustPackage {
  pname = "gulp";
  version = "0.1.0";

  src = builtins.path {
    path = ./.;
    name = "gulp-src";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
    makeWrapper
    llvmPackages.clang
    installShellFiles
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

    installShellCompletion --cmd gulp \
      --bash <($out/bin/gulp --generate-completions bash) \
      --fish <($out/bin/gulp --generate-completions fish) \
      --zsh <($out/bin/gulp --generate-completions zsh)
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
