{ pkgs }:
pkgs.stdenv.mkDerivation (finalAttrs: {
  pname = "BB_Launcher";
  version = "13.05";

  src = pkgs.fetchFromGitHub {
    owner = "rainmakerv3";
    repo = "BB_Launcher";
    tag = "Release${finalAttrs.version}";
    hash = "sha256-DRUyBpFyDsuLWbx3fUUZQVLxr6lwHmhE/U9kyGgi4gc=";
    fetchSubmodules = true;
  };

  nativeBuildInputs = [
    pkgs.cmake
    pkgs.pkg-config
    pkgs.wayland-scanner
    pkgs.qt6.wrapQtAppsHook
    pkgs.wrapGAppsHook3
  ];

  buildInputs = [
    pkgs.wayland
    pkgs.wayland-protocols
    pkgs.libxkbcommon
    pkgs.libGL
    pkgs.mesa
    pkgs.libX11
    pkgs.libXext
    pkgs.libXrandr
    pkgs.libXcursor
    pkgs.libXfixes
    pkgs.libXi
    pkgs.qt6.qtbase
    pkgs.qt6.qtwayland
    pkgs.qt6.qtwebview
    pkgs.gsettings-desktop-schemas
  ];

  postInstall = ''
    install -Dm644 $src/dist/BBLauncher.desktop \
      $out/share/applications/BBLauncher.desktop
    install -Dm644 $src/dist/BBIcon.png \
      $out/share/icons/hicolor/256x256/apps/BBIcon.png
  '';

  meta = {
    description = "Dedicated launcher/mod manager combo app for Bloodborne on shadPS4";
    homepage = "https://github.com/rainmakerv3/BB_Launcher";
    license = pkgs.lib.licenses.gpl3Only;
    mainProgram = "BB_Launcher";
    platforms = pkgs.lib.platforms.all;
  };
})
