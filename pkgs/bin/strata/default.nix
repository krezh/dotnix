{
  autoPatchelfHook,
  bubblewrap,
  cairo,
  fetchurl,
  ffmpeg,
  ffmpegthumbnailer,
  fontconfig,
  gdk-pixbuf,
  glib,
  graphene,
  gst_all_1,
  gtk4,
  gtksourceview5,
  lib,
  makeWrapper,
  pango,
  poppler,
  stdenv,
  wrapGAppsHook4,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "strata";
  # renovate: datasource=github-releases depName=lgse/strata
  version = "1.0.0";

  src = fetchurl {
    url = "https://github.com/lgse/strata/releases/download/v${finalAttrs.version}/strata-${finalAttrs.version}-x86_64-unknown-linux-gnu.tar.gz";
    hash = "sha256-4VCVogEWUTKogDLfFey+q3laE9bKzE67lD4nbDX/SS4=";
  };

  sourceRoot = "strata-${finalAttrs.version}-x86_64-unknown-linux-gnu";
  dontBuild = true;

  nativeBuildInputs = [
    autoPatchelfHook
    makeWrapper
    wrapGAppsHook4
  ];

  buildInputs = [
    cairo
    fontconfig
    gdk-pixbuf
    glib
    graphene
    gst_all_1.gst-plugins-base
    gst_all_1.gstreamer
    gtk4
    gtksourceview5
    pango
    poppler
    stdenv.cc.cc.lib
  ];

  installPhase = ''
    runHook preInstall

    install -Dm755 strata "$out/bin/strata"
    install -Dm644 io.github.lgse.Strata.desktop \
      "$out/share/applications/io.github.lgse.Strata.desktop"
    install -Dm644 io.github.lgse.Strata.svg \
      "$out/share/icons/hicolor/scalable/apps/io.github.lgse.Strata.svg"
    install -Dm644 SOURCE_COMMIT "$out/share/strata/SOURCE_COMMIT"

    substituteInPlace "$out/share/applications/io.github.lgse.Strata.desktop" \
      --replace-fail "Exec=strata " "Exec=$out/bin/strata "

    runHook postInstall
  '';

  preFixup = ''
    gappsWrapperArgs+=(
      --prefix PATH : ${
        lib.makeBinPath [
          bubblewrap
          ffmpeg
          ffmpegthumbnailer
        ]
      }
      --prefix GST_PLUGIN_SYSTEM_PATH_1_0 : ${
        lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" [
          gst_all_1.gst-libav
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
        ]
      }
    )
  '';

  passthru.sourceCommit = "6662d8d4bd4cb3cdec3f10f918f3b4c92b18cdcc";

  meta = {
    description = "Keyboard-first GTK4 file manager";
    homepage = "https://github.com/lgse/strata";
    changelog = "https://github.com/lgse/strata/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.mit;
    mainProgram = "strata";
    platforms = [ "x86_64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
})
