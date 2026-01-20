{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  name = "gulp";

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    cargo
    rustc
    rustfmt
    clippy
    wayland
    wayland-protocols
    wayland-scanner
    cairo
    pango
    libxkbcommon
    tesseract
    leptonica
    rust-analyzer
    llvmPackages.libclang
    llvmPackages.clang
  ];

  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.wayland}/lib/pkgconfig:${pkgs.wayland-protocols}/share/pkgconfig:${pkgs.cairo}/lib/pkgconfig:${pkgs.leptonica}/lib/pkgconfig:${pkgs.tesseract}/lib/pkgconfig:$PKG_CONFIG_PATH"
    export LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.cairo}/lib:${pkgs.libxkbcommon}/lib:${pkgs.leptonica}/lib:${pkgs.tesseract}/lib:$LD_LIBRARY_PATH"
    export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
    export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include -isystem ${pkgs.glibc.dev}/include"
  '';
}
