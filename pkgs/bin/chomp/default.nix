{
  lib,
  craneLib,
  pkgs,
  makeWrapper,
  llvmPackages,
  installShellFiles,
}:
craneLib.buildPackage rec {
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      (craneLib.fileset.commonCargoSources ./.)
      ./protocols/hyprland-toplevel-export-v1.xml
    ];
  };
  strictDeps = true;

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
    ffmpeg
    libdrm
    libva
    mesa
    pipewire
    x264
  ];

  env = {
    LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${llvmPackages.libclang.lib}/lib/clang/${llvmPackages.libclang.version}/include";
  };

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit
      src
      strictDeps
      buildInputs
      env
      ;
    nativeBuildInputs = with pkgs; [
      pkg-config
      llvmPackages.clang
    ];
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
    llvmPackages.clang
    makeWrapper
    installShellFiles
  ];

  postInstall = ''
    wrapProgram $out/bin/chomp \
      --prefix PATH : ${
        lib.makeBinPath [
          pkgs.tesseract
          pkgs.satty
          pkgs.wl-screenrec
          pkgs.wl-clipboard
        ]
      }

    installShellCompletion --cmd chomp \
      --bash <($out/bin/chomp --generate-completions bash) \
      --fish <($out/bin/chomp --generate-completions fish) \
      --zsh <($out/bin/chomp --generate-completions zsh)
  '';

  meta = {
    description = "Wayland screenshot, recording, OCR, upload, and native instant replay tool";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "chomp";
  };
}
