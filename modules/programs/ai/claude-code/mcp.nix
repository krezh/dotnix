{
  flake.modules.homeManager.ai =
    { pkgs, config, ... }:
    {
      programs.claude-code.mcpServers = {
        agentmemory = {
          type = "stdio";
          command = pkgs.writeShellScript "agentmemory-mcp-wrapper" ''
            export PATH="${pkgs.nodejs}/bin:$PATH"
            export AGENTMEMORY_URL="https://agentmemory-api.plexuz.xyz"
            export AGENTMEMORY_SECRET="$(cat ${config.sops.secrets."agentmemory/secret".path})"
            exec ${pkgs.nodejs}/bin/npx -y @agentmemory/mcp "$@"
          '';
        };
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
}
