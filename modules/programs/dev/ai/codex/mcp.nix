{
  flake.modules.homeManager.ai = {
    programs.codex.enableMcpIntegration = true;
    programs.codex.settings.mcp_servers.memini = {
      url = "https://memini.plexuz.xyz/mcp";
      bearer_token_env_var = "MEMINI_API_KEY";
    };
  };
}
