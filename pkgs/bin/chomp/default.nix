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
  pname = "chomp";
  version = "0.1.0";

  src = builtins.path {
    path = ./.;
    name = "chomp-src";
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
    openssl
  ];

  env = {
    LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${llvmPackages.libclang.lib}/lib/clang/${llvmPackages.libclang.version}/include";
  };

  postInstall = ''
    wrapProgram $out/bin/chomp \
      --prefix PATH : ${lib.makeBinPath [ pkgs.tesseract ]}

    installShellCompletion --cmd chomp \
      --bash <($out/bin/chomp --generate-completions bash) \
      --fish <($out/bin/chomp --generate-completions fish) \
      --zsh <($out/bin/chomp --generate-completions zsh)
  '';

  meta = {
    description = "A playful, compositor-agnostic Wayland screen selection tool with OCR support";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "chomp";
  };
}
