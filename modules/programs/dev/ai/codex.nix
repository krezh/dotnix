{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    { pkgs, ... }:
    let
      llm-agents-nix = inputs.llm-agents-nix.packages.${pkgs.stdenv.hostPlatform.system};
    in
    {
      programs.codex = {
        enable = true;
        package = llm-agents-nix.codex;

        settings = {
          mcp_servers = {
            mcp-tools = {
              enabled = true;
              url = "https://mcp.plexuz.xyz/mcp";
            };
          };
        };
      };
    };
}
