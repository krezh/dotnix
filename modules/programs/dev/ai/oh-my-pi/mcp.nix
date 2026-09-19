{
  flake.modules.homeManager.ai =
    { config, pkgs, ... }:
    let
      jsonFormat = pkgs.formats.json { };
    in
    {
      # Generate ~/.omp/agent/mcp.json connecting oh-my-pi to shared MCP servers
      home.file.".omp/agent/mcp.json".source = jsonFormat.generate "omp-mcp.json" {
        mcpServers = {
          konflate = {
            url = config.programs.mcp.servers.konflate.url;
          };
          mcp-tools = {
            url = config.programs.mcp.servers.mcp-tools.url;
          };
          memini = {
            url = "https://memini.plexuz.xyz/mcp";
            headers = {
              Authorization = "Bearer \${MEMINI_API_KEY}";
            };
          };
        };
      };
    };
}
