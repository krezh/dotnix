{
  flake.modules.homeManager.ai =
    { pkgs, ... }:
    {
      programs.claude-code.mcpServers = {
        nixos = {
          type = "stdio";
          command = "${pkgs.uv}/bin/uvx";
          args = [ "mcp-nixos" ];
        };
        mcp-tools = {
          type = "http";
          url = "https://mcp.plexuz.xyz/mcp";
        };
      };
    };
}
