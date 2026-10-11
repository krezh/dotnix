{
  flake.modules.homeManager.ai =
    { config, pkgs, ... }:
    let
      jsonFormat = pkgs.formats.json { };
    in
    {
      xdg.configFile."omp/agent/mcp.json".source = jsonFormat.generate "omp-mcp.json" {
        mcpServers = {
          konflate = {
            url = config.programs.mcp.servers.konflate.url;
          };
          mcp-tools = {
            url = config.programs.mcp.servers.mcp-tools.url;
          };
          memini = {
            url = "\${MEMINI_BASE_URL}/mcp";
            headers = {
              Authorization = "Bearer \${MEMINI_API_KEY}";
              X-Memini-Home = "\${MEMINI_HOME}";
            };
          };
        };
      };
    };
}
