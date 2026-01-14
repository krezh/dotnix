{
  flake.modules.nixos.ai =
    { pkgs, ... }:
    {
      environment.etc."claude-code/managed-mcp.json" = {
        text = builtins.toJSON {
          mcpServers = {
            nixos = {
              type = "stdio";
              command = "${pkgs.uv}/bin/uvx";
              args = [ "mcp-nixos" ];
            };
            forgetful = {
              type = "stdio";
              command = "${pkgs.uv}/bin/uvx";
              args = [ "forgetful-ai" ];
            };
            context7 = {
              type = "stdio";
              command = pkgs.writeShellScript "context7-mcp-wrapper" ''
                export PATH="${pkgs.nodejs}/bin:$PATH"
                exec ${pkgs.nodejs}/bin/npx -y @upstash/context7-mcp "$@"
              '';
            };
          };
        };
      };
    };
}
