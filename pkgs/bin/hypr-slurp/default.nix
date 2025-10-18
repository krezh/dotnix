{
  rustPlatform,
  pkgs,
  ...
}:

rustPlatform.buildRustPackage {
  pname = "hypr-slurp";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    wayland
    wayland-protocols
    wayland-scanner
    cairo
    pango
    libxkbcommon
  ];

  meta = {
    description = "A slurp-like region selector for Hyprland";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "hypr-slurp";
  };
}
