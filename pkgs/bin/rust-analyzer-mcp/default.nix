{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:

rustPlatform.buildRustPackage rec {
  pname = "rust-analyzer-mcp";
  # renovate: datasource=github-releases depName=zeenix/rust-analyzer-mcp
  version = "0.2.0";

  src = fetchFromGitHub {
    owner = "zeenix";
    repo = "rust-analyzer-mcp";
    rev = "v${version}";
    hash = "sha256-brnzVDPBB3sfM+5wDw74WGqN5ahtuV4OvaGhnQfDqM0=";
  };

  cargoHash = "sha256-7t4bjyCcbxFAO/29re7cjoW1ACieeEaM4+QT5QAwc34=";

  doCheck = false;

  meta = {
    description = "A Model Context Protocol (MCP) server that provides integration with rust-analyzer";
    homepage = "https://github.com/zeenix/rust-analyzer-mcp";
    license = lib.licenses.mit;
    mainProgram = "rust-analyzer-mcp";
  };
}
