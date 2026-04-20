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
    echo "Run 'nix build .#optiman' to build the project"
    echo "Run 'nix build .#optiman' to run the application"
  '';
}
