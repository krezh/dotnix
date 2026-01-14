{
  inputs,
  ...
}:
{
  flake.modules.homeManager.ai =
    { pkgs, ... }:
    let
      nix-ai-tools = inputs.nix-ai-tools.packages.${pkgs.stdenv.hostPlatform.system};
    in
    {
      programs.claude-code = {
        enable = true;
        package = nix-ai-tools.claude-code;
        mcpServers = { };

        settings = {
          theme = "dark";
          permissions = {
            allow = [
              # Safe read-only git commands
              "Bash(git add:*)"
              "Bash(git status)"
              "Bash(git log:*)"
              "Bash(git diff:*)"
              "Bash(git show:*)"
              "Bash(git branch:*)"
              "Bash(git remote:*)"

              # Safe Nix commands (mostly read-only)
              "Bash(nix:*)"

              # Safe programming language tools
              "Bash(cargo:*)"
              "Bash(go:*)"

              # Safe file system operations
              "Bash(ls:*)"
              "Bash(find:*)"
              "Bash(grep:*)"
              "Bash(rg:*)"
              "Bash(cat:*)"
              "Bash(head:*)"
              "Bash(tail:*)"
              "Bash(mkdir:*)"
              "Bash(chmod:*)"

              # Safe system info commands
              "Bash(systemctl list-units:*)"
              "Bash(systemctl list-timers:*)"
              "Bash(systemctl status:*)"
              "Bash(journalctl:*)"
              "Bash(dmesg:*)"
              "Bash(env)"
              "Bash(claude --version)"
              "Bash(nh search:*)"

              # Audio system (read-only)
              "Bash(pactl list:*)"
              "Bash(pw-top)"

              # Core Claude Code tools
              "Glob(*)"
              "Grep(*)"
              "LS(*)"
              "Read(*)"
              "Search(*)"
              "Task(*)"
              "TodoWrite(*)"

              # MCP servers (read-only)
              "mcp__github"
              "mcp__nixos"
              "mcp__rust-analyzer"
              "mcp__gopls"
              "mcp__sequential-thinking"
              "mcp__filesystem"

              # Safe web fetch from trusted domains
              "WebFetch(domain:wiki.hyprland.org)"
              "WebFetch(domain:wiki.hypr.land)"
              "WebFetch(domain:github.com)"
              "WebFetch(domain:raw.githubusercontent.com)"

              # NixOS build
              "Bash(nh os build:*)"
            ];
            deny = [
              "Bash(curl:*)"
              "Read(./.env)"
              "Read(./.env.*)"
              "Read(**/.secret*)"
              "Read(**/secret)"
              "Read(**/secret.*)"
              "Bash(sudo:*)"
            ];
            defaultMode = "acceptEdits";
          };
          model = "claude-sonnet-4-5";
          verbose = true;
          includeCoAuthoredBy = false;

          statusLine = {
            command = pkgs.writeShellScript "claude-powerline-wrapper" ''
              export PATH="${pkgs.nodejs}/bin:$PATH"
              ${pkgs.nodejs}/bin/npx -y @owloops/claude-powerline@latest --style=powerline
            '';
            padding = 0;
            type = "command";
          };
        };
      };
    };
}
