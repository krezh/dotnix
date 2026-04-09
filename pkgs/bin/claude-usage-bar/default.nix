{
  lib,
  craneLib,
}:
craneLib.buildPackage rec {
  src = craneLib.cleanCargoSource ./.;
  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly { inherit src strictDeps; };

  meta = {
    description = "Claude Code status line showing rate limit and context window usage";
    mainProgram = "claude-usage-bar";
    platforms = lib.platforms.unix;
  };
}
