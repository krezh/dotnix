{
  lib,
  craneLib,
}:
craneLib.buildPackage rec {
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      (craneLib.fileset.commonCargoSources ./.)
      ./70-zowie-batt.rules
    ];
  };
  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly { inherit src strictDeps; };

  postInstall = ''
    install -Dm444 70-zowie-batt.rules -t $out/lib/udev/rules.d
  '';

  meta = {
    description = "Read the battery voltage and charging state of BenQ ZOWIE DW wireless mice";
    mainProgram = "zowie-batt";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
  };
}
