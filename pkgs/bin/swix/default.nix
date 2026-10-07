{
  lib,
  craneLib,
  pkgs,
  nixPackage ? pkgs.nix,
  makeWrapper,
  wrapGAppsHook4,
}:
craneLib.buildPackage rec {
  src = lib.cleanSource ./.;
  strictDeps = true;

  buildInputs = with pkgs; [
    gtk4
    gtk4-layer-shell
    adwaita-icon-theme
  ];

  env = {
    SWIX_GIT = lib.getExe pkgs.git;
    SWIX_JJ = lib.getExe pkgs.jujutsu;
    SWIX_JOURNALCTL = lib.getExe' pkgs.systemd "journalctl";
    SWIX_NIX = lib.getExe nixPackage;
    SWIX_NIX_ENV = lib.getExe' nixPackage "nix-env";
    SWIX_NIX_STORE = lib.getExe' nixPackage "nix-store";
  };

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit
      src
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
      strictDeps
      buildInputs
      env
      cargoArtifacts
      ;
    nativeBuildInputs = with pkgs; [ pkg-config ];
    cargoClippyExtraArgs = "--all-targets -- --deny warnings";
  };

  preFixup = ''
    gappsWrapperArgs+=(
      --prefix PATH : ${
        lib.makeBinPath [
          pkgs.dix
          pkgs.git
          pkgs.jujutsu
          pkgs.nix-changelog
        ]
      }
    )
  '';

  meta = {
    description = "GTK control center for NixOS builds, updates, activation, and system cleanup";
    mainProgram = "swix";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
  };
}
