{
  inputs,
  pkgs,
  config,
  lib,
  ...
}:
{
  programs.claude-code = {
    enable = true;
    package = inputs.nix-ai-tools.packages.${pkgs.stdenv.hostPlatform.system}.claude-code;

    mcpServers = {
      github = {
        type = "stdio";
        command = lib.getExe pkgs.github-mcp-server;
        args = [
          "--read-only"
          "stdio"
        ];
        env = {
          GITHUB_PERSONAL_ACCESS_TOKEN = "$(cat ${config.sops.secrets."github/mcp_token".path})";
        };
      };
      nixos = {
        type = "stdio";
        command = lib.getExe pkgs.mcp-nixos;
      };
      socket = {
        type = "http";
        url = "https://mcp.socket.dev/";
      };
    };

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
          "mcp__socket"

          # Safe web fetch from trusted domains
          "WebFetch(domain:wiki.hyprland.org)"
          "WebFetch(domain:github.com)"
          "WebFetch(domain:wiki.hypr.land)"
          "WebFetch(domain:raw.githubusercontent.com)"
        ];
        ask = [
          # Potentially destructive git commands
          "Bash(git checkout:*)"
          "Bash(git commit:*)"
          "Bash(git merge:*)"
          "Bash(git pull:*)"
          "Bash(git push:*)"
          "Bash(git rebase:*)"
          "Bash(git reset:*)"
          "Bash(git restore:*)"
          "Bash(git stash:*)"
          "Bash(git switch:*)"

          # File deletion and modification
          "Bash(cp:*)"
          "Bash(mv:*)"
          "Bash(rm:*)"

          # System control operations
          "Bash(systemctl disable:*)"
          "Bash(systemctl enable:*)"
          "Bash(systemctl mask:*)"
          "Bash(systemctl reload:*)"
          "Bash(systemctl restart:*)"
          "Bash(systemctl start:*)"
          "Bash(systemctl stop:*)"
          "Bash(systemctl unmask:*)"

          # Network operations
          "Bash(curl:*)"
          "Bash(ping:*)"
          "Bash(rsync:*)"
          "Bash(scp:*)"
          "Bash(ssh:*)"
          "Bash(wget:*)"

          # Package management
          "Bash(nixos-rebuild:*)"

          # Process management
          "Bash(kill:*)"
          "Bash(killall:*)"
          "Bash(pkill:*)"
        ];
        deny = [
          "Bash(curl:*)"
          "Read(./.env)"
          "Read(./.env.*)"
          "Read(./secrets/**)"
          "Read(./secrets/**)"
          "Bash(sudo:*)"
        ];
        defaultMode = "acceptEdits";
      };
      model = "claude-sonnet-4-5";
      verbose = true;
      includeCoAuthoredBy = false;

      statusLine = {
        command = "${pkgs.bun}/bin/bunx ccusage statusline";
        padding = 0;
        type = "command";
      };
    };
  };
}
