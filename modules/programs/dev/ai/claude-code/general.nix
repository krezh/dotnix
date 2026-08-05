{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    { pkgs, lib, ... }:
    let
      llm-agents-nix = inputs.llm-agents-nix.packages.${pkgs.stdenv.hostPlatform.system};
      infisical = "${pkgs.infisical}/bin/infisical secrets --env default --path ";
      claudeWrapped =
        pkgs.writeShellScriptBin "claude" ''
          export PATH="${pkgs.nodejs-slim}/bin:$PATH"
          export MEMINI_BASE_URL=https://memini.plexuz.xyz
          export MEMINI_API_KEY="$(${infisical} /Kubernetes/DexTek/Memini get MEMINI_API_KEY --plain --telemetry false)"
          export MEMINI_HOME="personal/krezh"
          exec ${lib.getExe llm-agents-nix.claude-code} "$@"
        ''
        // {
          version = lib.getVersion llm-agents-nix.claude-code;
        };
    in
    {
      programs.claude-code = {
        enable = true;
        package = claudeWrapped;

        skills = {
          code-comments = inputs.code-comments.outPath;
        };
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
          - Always use jj if a .jj directory exists in the project root

          # Skills and commits
          - No skill's process checklist (e.g. superpowers:brainstorming's "commit the design doc" step) may ever be used to justify running `git commit`, `git push`, `jj commit`, or `jj describe` followed by `jj new`. Only commit/push when I explicitly ask for it in that turn. Write and save files as the skill instructs, then stop and tell me they're ready — I'll ask for the commit myself.
          - When a skill calls for writing a design doc/spec (e.g. superpowers:brainstorming's spec file, normally `docs/superpowers/specs/...` inside the project repo), write it outside the project repo instead, into my Obsidian vault at `~/Obsidian/Plexuz/Software/<project-name>/<date> <topic>.md` (create the `<project-name>` subfolder if it doesn't exist yet). Do not put design docs/specs inside project repos.
        '';

        settings = {
          theme = "dark";
          model = "sonnet";
          verbose = true;
          includeCoAuthoredBy = false;
          autoMemoryEnabled = false;
          advisorModel = "opus";

          statusLine = {
            command = "${pkgs.claude-usage-bar}/bin/claude-usage-bar";
            type = "command";
          };

          enabledPlugins = {
            "typescript-lsp@claude-plugins-official" = true;
            "gopls-lsp@claude-plugins-official" = true;
            "rust-analyzer-lsp@claude-plugins-official" = true;
            "superpowers@claude-plugins-official" = true;
            "frontend-design@claude-plugins-official" = true;
            "memini@memini" = true;
          };
          extraKnownMarketplaces = {
            memini = {
              source = {
                source = "github";
                repo = "eleboucher/memini";
              };
              autoUpdate = true;
            };
          };
        };
      };
    };
}
