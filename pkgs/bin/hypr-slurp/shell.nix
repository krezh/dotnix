{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  name = "hypr-slurp";

  buildInputs = with pkgs; [
    cargo
    rustc
    rustfmt
    clippy
    pkg-config
    wayland
    wayland-protocols
    wayland-scanner
    cairo
    pango
    libxkbcommon
    rust-analyzer
  ];

  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.wayland}/lib/pkgconfig:${pkgs.wayland-protocols}/share/pkgconfig:${pkgs.cairo}/lib/pkgconfig:$PKG_CONFIG_PATH"
    export LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.cairo}/lib:${pkgs.libxkbcommon}/lib:$LD_LIBRARY_PATH"
  '';
}
