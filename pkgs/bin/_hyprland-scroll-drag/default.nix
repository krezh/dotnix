{
  stdenv,
  cmake,
  pkg-config,
  hyprland,
}:
stdenv.mkDerivation {
  pname = "hyprland-scroll-drag";
  version = "0.1";
  src = ./.;

  nativeBuildInputs = [
    cmake
    pkg-config
  ];

  buildInputs = hyprland.buildInputs ++ [
    hyprland.dev
  ];
}
