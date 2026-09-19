{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    { pkgs, lib, ... }:
    let
      llm-agents-nix = inputs.llm-agents-nix.packages.${pkgs.stdenv.hostPlatform.system};
      antigravityWrapped =
        pkgs.writeShellScriptBin "agy" ''
          set -euo pipefail
          export PATH="${pkgs.infisical}/bin:$PATH"
          ${builtins.readFile ../lib/memini-env.sh}
          exec ${lib.getExe llm-agents-nix.antigravity-cli} "$@"
        ''
        // {
          version = lib.getVersion llm-agents-nix.antigravity-cli;
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
    in
    {
      programs.antigravity-cli = {
        enable = true;
        package = antigravityWrapped;

        context = {
          GEMINI = commonContext;
          AGENTS = commonContext;
        };

        settings = {
          allowNonWorkspaceAccess = true;
          model = "Gemini 3.8 Flash (Medium)";
          mode = "accept-edits";
          toolPermission = "proceed-in-sandbox";
          notifications = true;
        };
      };
    };
}
