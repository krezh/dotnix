{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "mcp-gopls";
  # renovate: datasource=github-releases depName=Yantrio/mcp-gopls
  version = "1.0.0";

  src = fetchFromGitHub {
    owner = "Yantrio";
    repo = "mcp-gopls";
    rev = "v${version}";
    hash = "sha256-Gk4xq5ZeeG/fshiDDqR4cMxmw6H0wCWxIrOer+SiALs=";
  };

  vendorHash = "sha256-A5I8RPFLE6joeh9jBHNyQhH74/5KBnCLeEbECY6oiwQ=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "Go language server (gopls) wrapped as an MCP server for AI-powered code assistance";
    homepage = "https://github.com/Yantrio/mcp-gopls";
    license = lib.licenses.mit;
    mainProgram = "mcp-gopls";
  };
}
