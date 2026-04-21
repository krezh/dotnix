{
  lib,
  craneLib,
  pkgs,
  makeWrapper,
}:
craneLib.buildPackage rec {
  src = craneLib.cleanCargoSource ./.;
  strictDeps = true;

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src strictDeps;
  };

  nativeBuildInputs = with pkgs; [ makeWrapper ];

  postInstall = ''
    wrapProgram $out/bin/nix-closure-diff \
      --prefix PATH : ${lib.makeBinPath [ pkgs.nix ]}
  '';

  meta = {
    description = "NixOS closure diff tool with JSON snapshot support for CI";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.gpl3;
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "nix-closure-diff";
  };
}
