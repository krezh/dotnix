{
  stdenv,
  cmake,
  pkg-config,
  hyprland,
  lua5_4,
  inputs,
}:
stdenv.mkDerivation {
  pname = "scrolloverview";
  version = "0.1";
  src = inputs.hyprland-scroll-overview;
  nativeBuildInputs = [
    cmake
    pkg-config
  ];
  buildInputs = hyprland.buildInputs ++ [
    hyprland.dev
    lua5_4
  ];
}
