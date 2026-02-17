{ pkgs }:
pkgs.stdenv.mkDerivation (finalAttrs: {
  pname = "shadps4-qtlauncher";
  version = "4QtLauncher-2026-02-10-088820e2c35974f64e995a9be4705646f4b48cd3";

  src = pkgs.fetchFromGitHub {
    owner = "shadps4-emu";
    repo = "shadps4-qtlauncher";
    tag = "shadPS${finalAttrs.version}";
    hash = "sha256-qYzsqMRdSES2vcDungp0QJh4x8Va1arr/SCI+sqz8mw=";
    fetchSubmodules = true;
  };

  nativeBuildInputs = [
    pkgs.cmake
    pkgs.pkg-config
    pkgs.qt6.wrapQtAppsHook
    pkgs.wrapGAppsHook3
  ];

  buildInputs = [
    pkgs.alsa-lib
    pkgs.libpulseaudio
    pkgs.openal
    pkgs.openssl
    pkgs.zlib
    pkgs.libedit
    pkgs.udev
    pkgs.libevdev
    pkgs.SDL2
    pkgs.jack2
    pkgs.sndio
    pkgs.qt6.qtbase
    pkgs.qt6.qttools
    pkgs.qt6.qtmultimedia
    pkgs.qt6.qtwayland
    pkgs.vulkan-headers
    pkgs.vulkan-utility-libraries
    pkgs.vulkan-tools
    pkgs.ffmpeg
    pkgs.fmt
    pkgs.glslang
    pkgs.libxkbcommon
    pkgs.wayland
    pkgs.wayland-protocols
    pkgs.libxcb
    pkgs.xcbutil
    pkgs.xcbutilkeysyms
    pkgs.xcbutilwm
    pkgs.stb
    pkgs.libpng
    pkgs.pipewire
    pkgs.gsettings-desktop-schemas
  ];

  qtWrapperArgs = [
    "--prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath [ pkgs.pipewire ]}"
  ];

  postInstall = ''
    install -Dm644 $src/dist/net.shadps4.shadps4-qtlauncher.desktop \
      $out/share/applications/net.shadps4.shadps4-qtlauncher.desktop
    install -Dm644 $src/src/images/net.shadps4.shadPS4.svg \
      $out/share/icons/hicolor/scalable/apps/net.shadps4.shadPS4.svg
  '';

  meta = {
    description = "The official Qt launcher for shadps4 emulator";
    homepage = "https://github.com/shadps4-emu/shadps4-qtlauncher";
    license = with pkgs.lib.licenses; [
      gpl2Only
      boost
      gpl2Plus
      mit
    ];
    mainProgram = "shadPS4QtLauncher";
    platforms = pkgs.lib.platforms.all;
  };
})
