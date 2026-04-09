{
  lib,
  craneLib,
  fetchFromGitHub,
  pkg-config,
  wrapGAppsHook4,
  cairo,
  gdk-pixbuf,
  glib,
  gtk4,
  gtk4-layer-shell,
  libpulseaudio,
  libxkbcommon,
  pango,
  sqlite,
  udev,
  stdenv,
  wayland,
  pkgs,
  rustPlatform,
}:
craneLib.buildPackage rec {
  pname = "wayle";
  version = "0.1.2";

  src = fetchFromGitHub {
    owner = "wayle-rs";
    repo = "wayle";
    tag = "v${version}";
    hash = "sha256-iZddhPdskoyyAYT3J92S5cRRKkkR8KyqIyBBPE+Lg18=";
  };

  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src pname strictDeps;
    nativeBuildInputs = [
      pkg-config
      rustPlatform.bindgenHook
    ];
    buildInputs = [
      cairo
      gdk-pixbuf
      glib
      gtk4
      gtk4-layer-shell
      libpulseaudio
      libxkbcommon
      pango
      sqlite
      udev
      pkgs.fftw
      pkgs.pipewire
    ]
    ++ lib.optionals stdenv.isLinux [ wayland ];
  };

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
    wrapGAppsHook4
  ];

  buildInputs = [
    cairo
    gdk-pixbuf
    glib
    gtk4
    gtk4-layer-shell
    libpulseaudio
    libxkbcommon
    pango
    sqlite
    udev
    pkgs.fftw
    pkgs.pipewire
  ]
  ++ lib.optionals stdenv.isLinux [ wayland ];

  cargoExtraArgs = "--package ${pname}";

  postInstall = ''
    install -dm755 $out/share/icons
    cp -r resources/icons/hicolor $out/share/icons/hicolor
    install -Dm644 resources/icons/index.theme $out/share/icons/index.theme
  '';

  meta = {
    description = "Wayland Elements -  A compositor agnostic shell with extensive customization";
    homepage = "https://github.com/wayle-rs/wayle";
    license = lib.licenses.mit;
    mainProgram = pname;
  };
}
