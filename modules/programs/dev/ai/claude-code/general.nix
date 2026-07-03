{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    { pkgs, lib, ... }:
    let
      llm-agents-nix = inputs.llm-agents-nix.packages.${pkgs.stdenv.hostPlatform.system};
      infisical = "${pkgs.infisical}/bin/infisical secrets --env default --path ";
      claudeWrapped = pkgs.writeShellScriptBin "claude" ''
        export PATH="${pkgs.nodejs-slim}/bin:$PATH"
        export MEMINI_URL=https://memini.plexuz.xyz
        export MEMINI_MCP_URL=https://memini.plexuz.xyz/mcp
        export MEMINI_TOKEN="$(${infisical} /Kubernetes/DexTek/Memini get MEMINI_API_KEY --plain --telemetry false)"
        exec ${lib.getExe llm-agents-nix.claude-code} "$@"
      '';
    in
    {
      programs.claude-code = {
        enable = true;
        package = claudeWrapped;
        context = ''
          # Personal preferences
          - I always run the latest versions of all software (this is a personal habit, not project-specific).
            When choosing config syntax, APIs, flags, or features, assume the newest release.
            Don't suggest legacy/older alternatives or hedge about version compatibility unless I ask.

          # Memory
          - Never read from or write to the built-in file-based auto-memory system (`~/.claude/projects/*/memory/`, `MEMORY.md`).
            Use the `memini` MCP plugin for all memory operations instead.
            If memini is unavailable, proceed without persistent memory rather than falling back to the file-based system.

          # Tools
          - Before assuming a capability isn't available, call the `mcp-tools` MCP server's `find_tool` to search its tool catalog, then `call_tool` to run whatever it finds.
            Do this proactively, without being asked, whenever a task needs something outside your built-in tools (infra/homelab integrations, etc.).
        '';

        settings = {
          theme = "dark";
          model = "sonnet";
          verbose = true;
          includeCoAuthoredBy = false;
          autoMemoryEnabled = false;

          statusLine = {
            command = "${pkgs.claude-usage-bar}/bin/claude-usage-bar";
            type = "command";
          };

          enabledPlugins = {
            "typescript-lsp@claude-plugins-official" = true;
            "gopls-lsp@claude-plugins-official" = true;
            "rust-analyzer-lsp@claude-plugins-official" = true;
            "superpowers@claude-plugins-official" = true;
            "memini@memini" = true;
          };
          extraKnownMarketplaces = {
            memini = {
              source = {
                source = "github";
                repo = "eleboucher/memini";
              };
            };
          };
        };
      };
    };
}
