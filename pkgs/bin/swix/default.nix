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
  ];

  env = {
    SWIX_NIX_ENV = lib.getExe' pkgs.nix "nix-env";
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
    makeWrapper
    wrapGAppsHook4
  ];

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
