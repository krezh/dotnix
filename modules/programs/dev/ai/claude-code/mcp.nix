{
  flake.modules.homeManager.ai = {
    programs.claude-code.mcpServers = {
      konflate = {
        type = "http";
        url = "https://konflate.plexuz.xyz/mcp";
      };
      mcp-tools = {
        type = "http";
        url = "https://mcp.plexuz.xyz/mcp";
      };
    };
  };
}
