{
  lib,
  stdenv,
  fetchFromGitHub,
  hyprland,
  hyprutils,
  hyprgraphics,
  hyprlang,
  aquamarine,
  pkg-config,
  pixman,
  libdrm,
  libglvnd,
  pango,
  cairo,
  libinput,
  systemd,
  wayland,
  libxkbcommon,
}:
stdenv.mkDerivation {
  pname = "hyprview";
  # renovate: datasource=github-releases depName=yz778/hyprview
  version = "v0.1.6";

  src = fetchFromGitHub {
    owner = "yz778";
    repo = "hyprview";
    rev = "f6eca5f3b27ab3c0739863e566100594019bfab7";
    hash = "sha256-XwtotbvzngRT2Pzc+LMHpNea7VZO5s19luvjwgymh0s=";
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    hyprland
    hyprutils
    hyprgraphics
    hyprlang
    aquamarine
    pixman
    libdrm
    libglvnd
    pango
    cairo
    libinput
    systemd
    wayland
    libxkbcommon
  ];

  buildPhase = ''
    runHook preBuild
    make -C src all
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/lib
    cp build/hyprview.so $out/lib/libhyprview.so
    runHook postInstall
  '';

  meta = with lib; {
    description = "Hyprland plugin for workspace overview";
    homepage = "https://github.com/yz778/hyprview";
    license = licenses.bsd3;
    platforms = platforms.linux;
    maintainers = [ ];
  };
}
