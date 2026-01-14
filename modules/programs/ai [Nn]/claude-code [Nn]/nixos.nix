{
  flake.modules.nixos.ai =
    { lib, pkgs, ... }:
    {
      environment.etc."claude-code/managed-mcp.json" = {
        text = builtins.toJSON {
          mcpServers = {
            github = {
              type = "stdio";
              command = pkgs.writeShellScript "github-mcp-wrapper" ''
                export GITHUB_PERSONAL_ACCESS_TOKEN="$(cat ~/.config/github/mcp_token)"
                exec ${lib.getExe pkgs.github-mcp-server} "$@"
              '';
              args = [
                "--read-only"
                "stdio"
              ];
            };
            nixos = {
              type = "stdio";
              command = "${pkgs.uv}/bin/uvx";
              args = [ "mcp-nixos" ];
            };
            filesystem = {
              type = "stdio";
              command = pkgs.writeShellScript "filesystem-mcp-wrapper" ''
                export PATH="${pkgs.nodejs}/bin:$PATH"
                exec ${pkgs.nodejs}/bin/npx -y @modelcontextprotocol/server-filesystem "$@"
              '';
            };
          };
        };
      };
    };
}
