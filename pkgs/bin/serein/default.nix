{
  lib,
  pkgs,
}:
let
  gstPlugins = with pkgs.gst_all_1; [
    gst-plugins-base
    gst-plugins-good
    gst-plugins-bad
    gst-libav
    gstreamer
    pkgs.pipewire
  ];

  runtimeLibraries = with pkgs; [
    vulkan-loader
    libGL
    libGLX
    libglvnd
    wayland
    libxkbcommon
    libx11
    libxi
    libxrandr
    libxcursor
  ];
in
pkgs.rustPlatform.buildRustPackage (finalAttrs: {
  pname = "serein";
  # renovate: datasource=github-releases depName=ViceVerse-cz/Serein
  version = "1.0.0-nightly.20261126.105";

  src = pkgs.fetchFromGitHub {
    owner = "ViceVerse-cz";
    repo = "Serein";
    tag = "v${finalAttrs.version}";
    hash = "sha256-1HoEaln++3KPUuPTytcRU2YarLdAqe6XFGf9nl/mJxk=";
  };

  cargoHash = "sha256-K3F91FiwKlFdzgZWCNdeNJor6KiETj45tWWAfCoVmhY=";

  env.RUSTFLAGS = "-C link-arg=-Wl,--exclude-libs,ALL";

  dontCargoUseSystemLibs = true;

  cargoBuildFlags = [
    "--package"
    "serein"
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
    cmake
    makeWrapper
    git
    rustPlatform.bindgenHook
    wrapGAppsHook4
    autoPatchelfHook
  ];

  buildInputs =
    with pkgs;
    [
      glib
      glib-networking
      gsettings-desktop-schemas
      gtk4
      webkitgtk_6_0
      cairo
      pango
      gdk-pixbuf
      graphene
      libsoup_3
      fontconfig
      libpulseaudio
      alsa-lib
    ]
    ++ runtimeLibraries
    ++ gstPlugins;

  runtimeDependencies = runtimeLibraries;

  doCheck = false;

  preFixup = ''
    gappsWrapperArgs+=(
      --prefix PATH : "${
        lib.makeBinPath [
          pkgs.bubblewrap
          pkgs.xdg-dbus-proxy
        ]
      }"
      --prefix GST_PLUGIN_SYSTEM_PATH_1_0 : "${lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" gstPlugins}"
    )
  '';

  postInstall = ''
    install -Dm444 packaging/linux/serein.desktop $out/share/applications/cz.viceverse.serein.desktop
    substituteInPlace $out/share/applications/cz.viceverse.serein.desktop \
      --replace-fail "Exec=serein" "Exec=$out/bin/serein"
    mkdir -p $out/share/icons
    cp -R packaging/linux/hicolor $out/share/icons/
  '';

  meta = {
    description = "Tiny, performant, native Discord client written in Rust (egui/wgpu)";
    homepage = "https://github.com/ViceVerse-cz/Serein";
    license = with lib.licenses; [
      mit
      asl20
    ];
    platforms = lib.platforms.linux;
    mainProgram = "serein";
  };
})
