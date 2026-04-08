{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "claude-usage-bar";
  version = "0.1.0";

  src = builtins.path {
    path = ./.;
    name = "claude-usage-bar-src";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  meta = {
    description = "Claude Code status line showing rate limit and context window usage";
    mainProgram = "claude-usage-bar";
    platforms = lib.platforms.unix;
  };
}
