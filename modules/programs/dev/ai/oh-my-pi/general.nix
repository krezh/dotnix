{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    { pkgs, lib, ... }:
    let
      llm-agents-nix = inputs.llm-agents-nix.packages.${pkgs.stdenv.hostPlatform.system};
      ompWrapped =
        pkgs.writeShellScriptBin "omp" ''
          set -euo pipefail
          export PATH="${pkgs.nodejs}/bin:${pkgs.bun}/bin:${pkgs.infisical}/bin:$PATH"
          export PI_CONFIG_DIR=".config/omp"
          ${builtins.readFile ../lib/memini-env.sh}
          exec ${lib.getExe llm-agents-nix.omp} "$@"
        ''
        // {
          version = lib.getVersion llm-agents-nix.omp;
        };

      commonContext = ''
        # Personal preferences
        - I always run the latest versions of all software (this is a personal habit, not project-specific).
          When choosing config syntax, APIs, flags, or features, assume the newest release.
          Don't suggest legacy/older alternatives or hedge about version compatibility unless I ask.

        # Memory
        - Use the `memini` MCP server for all persistent cross-session memory operations.
          If memini is unavailable, proceed without persistent memory.

        # Tools
        - Before assuming a capability isn't available, check the mcp-tools MCP server's tool catalog, then call whatever it finds.
          Do this proactively, without being asked, whenever a task needs something outside your built-in tools (infra/homelab integrations, etc.).
        - Always use jj if a .jj directory exists in the project root
      '';

      yamlFormat = pkgs.formats.yaml { };
      ompConfig = {
        modelRoles = {
          default = "openai-codex/gpt-5.6-sol:medium";
        };
        symbolPreset = "nerd";
        composer = {
          shape = "claude";
        };
        theme = {
          dark = "titanium";
          light = "light";
        };
        setupVersion = 2;
        statusLine = {
          preset = "custom";
          transparent = true;
          separator = "powerline-thin";
          leftSegments = [
            "pi"
            "vim"
            "model"
            "usage"
            "mode"
            "collab"
            "stream"
            "path"
            "git"
            "pr"
            "context_pct"
            "cost"
          ];
          rightSegments = [
            "session_name"
          ];
        };
        terminal = {
          showProgress = true;
        };
        images = {
          blockImages = true;
        };
        retry = {
          waitForUsageReset = true;
        };
        github = {
          enabled = true;
        };
        dev = {
          autoqaConsent = "denied";
        };
      };
    in
    {
      home.packages = [
        ompWrapped
        (pkgs.writeShellScriptBin "oh-my-pi" ''
          exec ${ompWrapped}/bin/omp "$@"
        '')
      ];

      xdg.configFile = {
        "omp/agent/AGENTS.md".text = commonContext;
        "omp/agent/config.yaml".source = yamlFormat.generate "omp-config.yaml" ompConfig;
      };
    };
}
