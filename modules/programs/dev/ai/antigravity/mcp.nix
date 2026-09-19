{
  flake.modules.homeManager.ai = {
    # konflate/mcp-tools are declared once in ../mcp.nix via the shared
    # programs.mcp module; this opts antigravity-cli into that same config.
    programs.antigravity-cli = {
      enableMcpIntegration = true;
      mcpServers.memini = {
        url = "https://memini.plexuz.xyz/mcp";
      };
    };
  };
}
