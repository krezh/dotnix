{ inputs, ... }:
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

        settings = {
          theme = "dark";
          model = "claude-sonnet-4-5";
          verbose = true;
          includeCoAuthoredBy = false;

          statusLine = {
            command = "${pkgs.claude-usage-bar}/bin/claude-usage-bar";
            type = "command";
          };

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
              "Web Search(*)"
              "Task(*)"
              "TodoWrite(*)"

              # MCP servers
              "mcp__nixos"

              # Safe web fetch from trusted domains
              "WebFetch(domain:wiki.hyprland.org)"
              "WebFetch(domain:wiki.hypr.land)"
              "WebFetch(domain:github.com)"
              "WebFetch(domain:raw.githubusercontent.com)"
              "WebFetch(domain:docs.renovatebot.com)"

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
        };
      };
    };
}
