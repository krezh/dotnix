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
        settings = {
          theme = "dark";
          model = "claude-sonnet-4-6";
          verbose = true;
          includeCoAuthoredBy = false;

          statusLine = {
            command = "${pkgs.claude-usage-bar}/bin/claude-usage-bar";
            type = "command";
          };

          # User memory/instructions
          memory.text = ''
            # Working relationship
            - No sycophancy.
            - Be direct, matter-of-fact, and concise.
            - Be critical; challenge my reasoning.
            - Don't include timeline estimates in plans.

            # Tooling

            ## General
            - Use your Edit tool for changes; Search tool for searching.
            - Use Mermaid diagrams for complex systems.

            ## Git
            - When creating git commit messages ALWAYS use [conventional commit style](https://www.conventionalcommits.org/en/v1.0.0/#specification).
            - When creating pull requests in Github ALWAYS mark them in draft status.
            - When interacting with Github (pull request actions, comments etc.) ALWAYS prefer using the `gh` CLI over the Github MCP or other actions.
          '';

          enabledPlugins = {
            "typescript-lsp@claude-plugins-official" = true;
            "gopls-lsp@claude-plugins-official" = true;
            "rust-analyzer-lsp@claude-plugins-official" = true;
            "superpowers@superpowers-marketplace" = true;
            "frontend-design@claude-plugins-official" = true;
            "memini@memini" = true;
          };
          extraKnownMarketplaces = {
            superpowers-marketplace = {
              source = {
                source = "github";
                repo = "obra/superpowers-marketplace";
              };
            };
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
