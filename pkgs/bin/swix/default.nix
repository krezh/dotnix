{
  lib,
  craneLib,
  pkgs,
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
    SWIX_NIX_ENV = lib.getExe' pkgs.nix "nix-env";
    SWIX_GIT = lib.getExe pkgs.git;
    SWIX_JJ = lib.getExe pkgs.jujutsu;
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
          pkgs.nix
          pkgs.nix-changelog
        ]
      }
    )
  '';

  meta = {
    description = "GTK software updates, changelogs, and switching for NixOS and Home Manager";
    mainProgram = "swix";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
  };
}
