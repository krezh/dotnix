{
  lib,
  pkgs,
}:
let
  id = "io.github.yapfast.Yapfast";

  webrtc = pkgs.runCommand "livekit-libwebrtc-51ef663" { nativeBuildInputs = [ pkgs.unzip ]; } ''
    unzip -q ${
      pkgs.fetchurl {
        url = "https://github.com/livekit/rust-sdks/releases/download/webrtc-51ef663/webrtc-linux-x64-release.zip";
        hash = "sha256-3OnZQUzY4syaRxwFRIGFGyV60qh5ESzlvlY2m0/v2u0=";
      }
    } -d unpacked
    mv unpacked/linux-x64-release "$out"
  '';

  libraries = with pkgs; [
    alsa-lib
    libpulseaudio
    fontconfig
    freetype
    glib
    libxkbcommon
    openssl
    wayland
    libGL
    libx11
    libxcursor
    libxi
    libxrandr
    libxcb
    pipewire
    libdrm
    libgbm
    libxfixes
    libxdamage
    libxcomposite
    libxext
  ];

  runtimeLibraries = libraries ++ [
    pkgs.libva
    pkgs.vulkan-loader
  ];
in
pkgs.rustPlatform.buildRustPackage (finalAttrs: {
  pname = "yapfast";
  # renovate: datasource=github-releases depName=yapfast/yapfast
  version = "0.5.0-nightly.20261128.22";

  src = pkgs.fetchFromGitHub {
    owner = "yapfast";
    repo = "yapfast";
    tag = "v${finalAttrs.version}";
    hash = "sha256-rTA1a0e3jb4x9uyPLi/EjfnQvhHyO0OLFIWRgAWzYXc=";
  };

  cargoHash = "sha256-UQVw1yWRlkzwmoFryqFkNfZsVrjpVeBiZS8tZEtYT24=";

  buildNoDefaultFeatures = true;
  buildFeatures = [
    "desktop"
    "discord"
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
    cmake
    llvmPackages.clang
    llvmPackages.llvm
    protobuf
    nasm
    makeWrapper
    rustPlatform.bindgenHook
  ];

  buildInputs = libraries ++ [ pkgs.aws-lc ];

  dontUseCmakeConfigure = true;

  env = {
    CC = "clang";
    CXX = "clang++";
    PROTOC = lib.getExe pkgs.protobuf;
    LK_CUSTOM_WEBRTC = "${webrtc}";
    YAPFAST_RELEASE = "1";
  };

  doCheck = false;

  stripAllList = [ "bin" ];

  postInstall = ''
    install -Dm644 packaging/icon.svg "$out/share/icons/hicolor/scalable/apps/${id}.svg"
    install -Dm644 packaging/linux/${id}.desktop -t "$out/share/applications"
    install -Dm644 packaging/linux/${id}.metainfo.xml -t "$out/share/metainfo"
    wrapProgram "$out/bin/yapfast" \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibraries} \
      --suffix LD_LIBRARY_PATH : /run/opengl-driver/lib
  '';

  meta = {
    description = "Native Discord client that stays light on memory";
    homepage = "https://github.com/yapfast/yapfast";
    license = lib.licenses.gpl3Plus;
    mainProgram = "yapfast";
    platforms = [ "x86_64-linux" ];
  };
})
