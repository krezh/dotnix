{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.nixosModules.claude-code;
in
{
  options.nixosModules.claude-code = {
    enable = lib.mkEnableOption "claude-code";
  };

  config = lib.mkIf cfg.enable {
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
            command = lib.getExe pkgs.mcp-nixos;
          };
          rust-analyzer = {
            type = "stdio";
            command = lib.getExe pkgs.rust-analyzer-mcp;
          };
          gopls = {
            type = "stdio";
            command = lib.getExe pkgs.mcp-gopls;
          };
          sequential-thinking = {
            type = "stdio";
            command = pkgs.writeShellScript "sequential-thinking-mcp-wrapper" ''
              export PATH="${pkgs.nodejs}/bin:$PATH"
              exec ${pkgs.nodejs}/bin/npx -y @modelcontextprotocol/server-sequential-thinking "$@"
            '';
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
