{
  lib,
  craneLib,
}:
craneLib.buildPackage rec {
  src = craneLib.cleanCargoSource ./.;
  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly { inherit src strictDeps; };

  meta = {
    description = "test";
    mainProgram = "test";
    platforms = lib.platforms.unix;
  };
}
