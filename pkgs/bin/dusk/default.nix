{
  lib,
  craneLib,
  pkgs,
}:
let
  common = {
    src = lib.cleanSource ./.;
    strictDeps = true;
    buildInputs = with pkgs; [
      gtk4
      gtk4-layer-shell
      adwaita-icon-theme
    ];
    nativeBuildInputs = with pkgs; [ pkg-config ];
  };
  cargoArtifacts = craneLib.buildDepsOnly common;
in
craneLib.buildPackage (
  common
  // {
    inherit cargoArtifacts;
    nativeBuildInputs = common.nativeBuildInputs ++ [ pkgs.wrapGAppsHook4 ];
    preFixup = ''
      gappsWrapperArgs+=(--prefix PATH : ${
        lib.makeBinPath [
          pkgs.systemd
          pkgs.kbd
        ]
      })
    '';
    passthru.tests.clippy = craneLib.cargoClippy (
      common
      // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      }
    );
    meta = {
      description = "A graceful session exit interface for Hyprland";
      mainProgram = "dusk";
      license = lib.licenses.gpl3Only;
      platforms = lib.platforms.linux;
    };
  }
)
