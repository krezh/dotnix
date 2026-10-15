{
  lib,
  craneLib,
  pkgs,
  makeWrapper,
  wrapGAppsHook4,
}:
craneLib.buildPackage rec {
  # The root spans `pkgs/bin` so the `nix-changelog` path dependency is in the source.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      (lib.fileset.fromSource (lib.cleanSource ./.))
      (craneLib.fileset.commonCargoSources ../nix-changelog)
    ];
  };
  cargoToml = ./Cargo.toml;
  cargoLock = ./Cargo.lock;
  postUnpack = ''
    cd "$sourceRoot/molt"
    sourceRoot="."
  '';
  strictDeps = true;

  buildInputs = with pkgs; [
    gtk4
    gtk4-layer-shell
    adwaita-icon-theme
  ];

  env = {
    MOLT_GIT = lib.getExe pkgs.git;
    MOLT_JJ = lib.getExe pkgs.jujutsu;
  };

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit
      src
      cargoToml
      cargoLock
      postUnpack
      strictDeps
      buildInputs
      env
      ;
    nativeBuildInputs = with pkgs; [ pkg-config ];
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
    git
    makeWrapper
    wrapGAppsHook4
  ];


  preCheck = ''
    export HOME="$TMPDIR/home"
    export XDG_CACHE_HOME="$HOME/.cache"
    export XDG_CONFIG_HOME="$HOME/.config"
    mkdir -p "$XDG_CACHE_HOME" "$XDG_CONFIG_HOME"
  '';
  cargoTestExtraArgs = "--all-targets";

  passthru.tests.clippy = craneLib.cargoClippy {
    inherit
      src
      cargoToml
      cargoLock
      postUnpack
      strictDeps
      buildInputs
      env
      cargoArtifacts
      ;
    nativeBuildInputs = with pkgs; [ pkg-config ];
    cargoClippyExtraArgs = "--all-targets -- --deny warnings";
  };

  postInstall = ''
    install -Dm644 ${./data/io.github.krezh.Molt.desktop} \
      "$out/share/applications/io.github.krezh.Molt.desktop"
    install -Dm644 ${./data/io.github.krezh.Molt.svg} \
      "$out/share/icons/hicolor/scalable/apps/io.github.krezh.Molt.svg"
    install -Dm644 ${./data/io.github.krezh.Molt-symbolic.svg} \
      "$out/share/icons/hicolor/symbolic/apps/io.github.krezh.Molt-symbolic.svg"
  '';

  preFixup = ''
    gappsWrapperArgs+=(
      --prefix PATH : ${
        lib.makeBinPath [
          pkgs.git
          pkgs.jujutsu
        ]
      }
    )
  '';

  meta = {
    description = "GTK control center for NixOS builds, updates, activation, and system cleanup";
    mainProgram = "molt";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
  };
}
