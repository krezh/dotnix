{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    { pkgs, config, ... }:
    let
      nix-ai-tools = inputs.nix-ai-tools.packages.${pkgs.stdenv.hostPlatform.system};
      tokenPath = config.sops.secrets."agentmemory/token".path;
      commandsSrc = "${pkgs.agentmemory}/plugins/opencode/commands";

      opencodeWrapped = pkgs.writeShellScriptBin "opencode" ''
        export AGENTMEMORY_URL="https://agentmemory.plexuz.xyz"
        export AGENTMEMORY_SECRET="$(cat ${tokenPath})"
        export AGENTMEMORY_FORCE_PROXY=1
        export AGENTMEMORY_TOOLS=all
        export OPENCODE_AGENTMEMORY_DEBUG=1
        exec ${nix-ai-tools.opencode}/bin/opencode "$@"
      '';

      mcpWrapper = pkgs.writeShellScript "agentmemory-mcp-wrapper" ''
        exec ${pkgs.nodejs}/bin/node ${pkgs.agentmemory}/dist/standalone.mjs "$@"
      '';
    in
    {
      programs.opencode = {
        enable = true;
        package = opencodeWrapped;

        tui = {
          scroll_speed = 3;
          scroll_acceleration = {
            enabled = true;
          };
        };

        settings = {
          model = "anthropic/claude-sonnet-4-5";
          small_model = "anthropic/claude-haiku-4";
          share = "manual";
          autoupdate = false;

          lsp = true;

          # Permission configuration
          # OpenCode uses per-tool permissions with states: "allow", "ask", or "deny"
          # For bash: the last matching rule takes precedence, so wildcards should be listed first
          permission = {
            # Core tool permissions
            edit = "ask";
            read = "allow";
            glob = "allow";
            grep = "allow";
            list = "allow";
            task = "allow";
            todowrite = "allow";
            todoread = "allow";
            question = "allow";
            webfetch = "allow";

            # Bash command permissions with pattern matching
            bash = {
              # Wildcard first (lowest precedence)
              "*" = "ask";

              # Deny dangerous commands (override wildcard)
              "curl *" = "deny";
              "sudo *" = "deny";

              # Allow safe git commands
              "git add *" = "allow";
              "git status" = "allow";
              "git log *" = "allow";
              "git diff *" = "allow";
              "git show *" = "allow";
              "git branch *" = "allow";
              "git remote *" = "allow";

              # Allow safe Nix commands
              "nix *" = "allow";
              "nh search *" = "allow";
              "nh os build *" = "allow";

              # Allow safe programming tools
              "cargo *" = "allow";
              "go *" = "allow";

              # Allow safe file system operations
              "ls *" = "allow";
              "find *" = "allow";
              "grep *" = "allow";
              "rg *" = "allow";
              "cat *" = "allow";
              "head *" = "allow";
              "tail *" = "allow";
              "mkdir *" = "allow";
              "chmod *" = "allow";

              # Allow safe system info commands
              "systemctl list-units *" = "allow";
              "systemctl list-timers *" = "allow";
              "systemctl status *" = "allow";
              "journalctl *" = "allow";
              "dmesg *" = "allow";
              "env" = "allow";
              "opencode --version" = "allow";

              # Allow audio system commands
              "pactl list *" = "allow";
              "pw-top" = "allow";
            };
          };

          plugin = [
            "opencode-mem"
            "opentmux"
            "opencode/plugins/agentmemory-capture.ts"
          ];

          # MCP server configuration
          mcp = {
            nixos = {
              enabled = true;
              type = "local";
              command = [
                "${pkgs.uv}/bin/uvx"
                "mcp-nixos"
              ];
            };

            context7 = {
              enabled = true;
              type = "local";
              command = [
                "${pkgs.writeShellScript "context7-mcp-wrapper" ''
                  export PATH="${pkgs.nodejs}/bin:$PATH"
                  exec ${pkgs.nodejs}/bin/npx -y @upstash/context7-mcp "$@"
                ''}"
              ];
            };

            agentmemory = {
              enabled = true;
              type = "local";
              command = [ mcpWrapper ];
            };
          };
        };
      };

      home.file = {
        ".config/opencode/plugins/agentmemory-capture.ts".source =
          "${pkgs.agentmemory}/plugins/opencode/agentmemory-capture.ts";
        ".config/opencode/commands/recall.md".source = "${commandsSrc}/recall.md";
        ".config/opencode/commands/remember.md".source = "${commandsSrc}/remember.md";
      };
    };
}
