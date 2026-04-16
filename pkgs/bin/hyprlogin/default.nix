{ pkgs, ... }:
pkgs.gcc15Stdenv.mkDerivation {
  pname = "hyprlogin";
  # renovate: datasource=github-releases depName=AuthenticSm1les/hyprlogin
  version = "0-unstable-2026-04-13";

  src = pkgs.fetchFromGitHub {
    owner = "AuthenticSm1les";
    repo = "hyprlogin";
    rev = "d229ed61777f5564cb7c7934b5c86a2b6ba39ffd";
    hash = "sha256-U/GByreAuZ1tkyUPrBacIi/M/hMV3yVSZYsF5LsusCU=";
  };

  nativeBuildInputs = with pkgs; [
    cmake
    pkg-config
    hyprwayland-scanner
    wayland-scanner
  ];

  buildInputs = with pkgs; [
    cairo
    hyprgraphics
    hyprlang
    hyprutils
    libdrm
    libgbm
    libGL
    libxkbcommon
    pam
    pango
    sdbus-cpp_2
    systemdLibs
    wayland
    wayland-protocols
  ];

  cmakeFlags = [ "-DCMAKE_BUILD_TYPE=Release" ];

  meta = with pkgs.lib; {
    description = "A Wayland greeter for greetd based on hyprlock";
    homepage = "https://github.com/AuthenticSm1les/hyprlogin";
    license = licenses.bsd3;
    mainProgram = "hyprlogin";
  };
}
