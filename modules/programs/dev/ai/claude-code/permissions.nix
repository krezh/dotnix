{
  flake.modules.homeManager.ai = {
    programs.claude-code = {
      settings = {
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
            "Bash(git rev-parse:*)"
            "Bash(git blame:*)"

            # Safe jj commands (read-only)
            "Bash(jj log:*)"
            "Bash(jj diff:*)"
            "Bash(jj status)"
            "Bash(jj st)"
            "Bash(jj show:*)"
            "Bash(jj op log:*)"
            "Bash(jj file show:*)"

            # Safe jj write commands
            "Bash(jj restore:*)"
            "Bash(jj describe:*)"

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
            "mcp__ide__*"
            "mcp__plugin_claude-code-home-manager_context7__*"
            "mcp__plugin_claude-code-home-manager_nixos__*"
            "mcp__plugin_claude-code-home-manager_mcp-tools__*"

            # Safe web fetch from trusted domains
            "WebFetch(domain:wiki.hyprland.org)"
            "WebFetch(domain:wiki.hypr.land)"
            "WebFetch(domain:github.com)"
            "WebFetch(domain:raw.githubusercontent.com)"
            "WebFetch(domain:*renovatebot.com)"

            # GitHub CLI read-only
            "Bash(gh search *)"
            "Bash(gh api *)"

            # NixOS build
            "Bash(nh os build:*)"
            "Bash(nixos-rebuild build:*)"
            "Bash(nix build:*)"

            # Kubernetes read-only
            "Bash(kubectl get *)"
            "Bash(kubectl logs *)"
            "Bash(kubectl describe *)"

            # Security/lint tools
            "Bash(shellcheck *)"
            "Bash(zizmor *)"
          ];
          deny = [
            "Bash(kubectl get secret* -o *)"
            "Bash(kubectl get secrets* -o *)"
            "Bash(curl:*)"
            "Bash(sed:*)"
            "Read(**/.env.*)"
            "Read(**/.secret*)"
            "Read(**/secret)"
            "Read(**/secret.*)"
            "Read(**/.decrypted~secrets.sops.*)"
            "Bash(sudo:*)"
            "Bash(nh os switch:*)"
          ];
          defaultMode = "acceptEdits";
        };
      };
    };
  };
}
