{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    cargo
    rustc
    rustfmt
    clippy
    rust-analyzer

    # Build dependencies
    pkg-config
    gtk4
    libadwaita
    openssl

    # Development tools
    gdb
  ];

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  shellHook = ''
    echo "OptiMan development environment"
    echo "Run 'cargo build' to build the project"
    echo "Run 'cargo run' to run the application"
  '';
}
