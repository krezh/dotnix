{
  lib,
  pkgs,
}:
pkgs.craneLib.buildPackage rec {
  src = pkgs.craneLib.cleanCargoSource ./.;
  strictDeps = true;
  cargoArtifacts = pkgs.craneLib.buildDepsOnly { inherit src strictDeps; };

  meta = {
    description = "Claude Code status line showing rate limit and context window usage";
    mainProgram = "claude-usage-bar";
    platforms = lib.platforms.unix;
  };
}
