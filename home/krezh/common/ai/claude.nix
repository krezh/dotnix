{
  inputs,
  pkgs,
  config,
  lib,
  ...
}:
let
  nix-ai-tools = inputs.nix-ai-tools.packages.${pkgs.stdenv.hostPlatform.system};
  github-mcp-wrapper = pkgs.writeShellScript "github-mcp-wrapper" ''
    export GITHUB_PERSONAL_ACCESS_TOKEN="$(cat ${config.sops.secrets."github/mcp_token".path})"
    exec ${lib.getExe pkgs.github-mcp-server} "$@"
  '';
in
{
  home.packages = [
    nix-ai-tools.claude-desktop
  ];

  xdg.desktopEntries = {
    claude-desktop = {
      name = "Claude Desktop";
      comment = "AI assistant with advanced reasoning capabilities";
      exec = "${lib.getExe nix-ai-tools.claude-desktop}";
      icon = "${nix-ai-tools.claude-desktop}/lib/claude-desktop/resources/claude-screen.png";
      terminal = false;
      categories = [
        "Development"
        "Chat"
        "Network"
      ];
      settings = {
        Keywords = "ai;assistant;chat;claude;anthropic;";
        StartupWMClass = "claude-desktop";
      };
    };

    claude = {
      name = "Claude Code";
      comment = "Claude CLI for code assistance";
      exec = "${lib.getExe nix-ai-tools.claude-code}";
      icon = "${nix-ai-tools.claude-desktop}/lib/claude-desktop/resources/claude-screen.png";
      terminal = true;
      categories = [
        "Development"
        "ConsoleOnly"
      ];
      settings = {
        Keywords = "ai;assistant;cli;claude;code;terminal;";
        StartupWMClass = "claude";
      };
    };
  };

  programs.claude-code = {
    enable = true;
    package = nix-ai-tools.claude-code;

    mcpServers = {
      github = {
        type = "stdio";
        command = "${github-mcp-wrapper}";
        args = [
          "--read-only"
          "stdio"
        ];
      };
      nixos = {
        type = "stdio";
        command = lib.getExe pkgs.mcp-nixos;
      };
      socket = {
        type = "http";
        url = "https://mcp.socket.dev/";
      };
      rust-analyzer = {
        type = "stdio";
        command = lib.getExe pkgs.rust-analyzer-mcp;
      };
      gopls = {
        type = "stdio";
        command = lib.getExe pkgs.mcp-gopls;
      };
      grafana = {
        type = "stdio";
        command = lib.getExe pkgs.mcp-grafana;
      };
      sequential-thinking = {
        type = "stdio";
        command = "${pkgs.bun}/bin/bunx";
        args = [
          "@modelcontextprotocol/server-sequential-thinking"
        ];
      };
      filesystem = {
        type = "stdio";
        command = "${pkgs.bun}/bin/bunx";
        args = [
          "@modelcontextprotocol/server-filesystem"
          # "${config.home.homeDirectory}/nix-config"
          # "${config.home.homeDirectory}/repos"
        ];
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
          "mcp__socket"
          "mcp__rust-analyzer"
          "mcp__gopls"
          "mcp__sequential-thinking"
          "mcp__filesystem"
          "mcp__grafana"

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
