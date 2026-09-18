{
  flake.modules.homeManager.ai = {
    # konflate/mcp-tools are declared once in ../mcp.nix via the shared
    # programs.mcp module; this just opts opencode into that same config.
    programs.opencode.enableMcpIntegration = true;
  };
}
