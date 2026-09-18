{
  flake.modules.homeManager.ai = {
    # Shared MCP servers consumed by every agent below via their own
    # `enableMcpIntegration` option (claude-code/mcp.nix, codex/mcp.nix,
    # opencode.nix). Agent-specific servers that need features this shared
    # schema can't express (e.g. codex's memini entry, which needs
    # `bearer_token_env_var`) stay declared directly in that agent's own file.
    programs.mcp = {
      enable = true;
      servers = {
        konflate.url = "https://konflate.plexuz.xyz/mcp";
        mcp-tools.url = "https://mcp.plexuz.xyz/mcp";
      };
    };
  };
}
