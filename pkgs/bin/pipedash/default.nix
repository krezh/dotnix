{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
  dpkg,
  wrapGAppsHook3,
  atk,
  cairo,
  gdk-pixbuf,
  glib,
  gtk3,
  libayatana-appindicator,
  libsoup_3,
  openssl,
  pango,
  sqlite,
  webkitgtk_4_1,
  wayland,
}:

stdenv.mkDerivation rec {
  pname = "pipedash";
  version = "0.0.8";

  src = fetchurl {
    url = "https://github.com/hcavarsan/pipedash/releases/download/v${version}/pipedash_${version}_amd64.deb";
    hash = "sha256-ax5Ok14f/IAhMQoWpyEQH+Xq0LAolPswh2NFkT8+Erc=";
  };

  nativeBuildInputs = [
    autoPatchelfHook
    dpkg
    wrapGAppsHook3
  ];

  buildInputs = [
    atk
    cairo
    gdk-pixbuf
    glib
    gtk3
    libayatana-appindicator
    libsoup_3
    openssl
    pango
    sqlite
    webkitgtk_4_1
    wayland
  ];

  unpackPhase = ''
    runHook preUnpack
    dpkg-deb -x $src .
    runHook postUnpack
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin
    cp -r usr/bin/* $out/bin/

    mkdir -p $out/share
    cp -r usr/share/* $out/share/

    runHook postInstall
  '';

  postFixup = ''
    wrapProgram $out/bin/pipedash \
      --set WEBKIT_DISABLE_DMABUF_RENDERER 1
  '';

  meta = {
    description = "A desktop app for managing CI/CD pipelines from multiple providers";
    homepage = "https://github.com/hcavarsan/pipedash";
    license = lib.licenses.gpl3Only;
    mainProgram = "pipedash";
    platforms = [ "x86_64-linux" ];
    sourceProvenance = with lib.sourceTypes; [ binaryNativeCode ];
  };
}
