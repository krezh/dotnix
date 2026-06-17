{
  flake.modules.homeManager.ai = {
    programs.claude-code.mcpServers = {
      mcp-tools = {
        type = "http";
        url = "https://mcp.plexuz.xyz/mcp";
      };
    };
  };
}
