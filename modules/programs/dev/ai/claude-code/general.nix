{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      llm-agents-nix = inputs.llm-agents-nix.packages.${pkgs.stdenv.hostPlatform.system};
      claudeWrapped =
        pkgs.writeShellScriptBin "claude" ''
          set -euo pipefail
          export CLAUDE_CODE_THRIFTY_SONIC=0
          export PATH="${pkgs.nodejs-slim}/bin:${pkgs.infisical}/bin:$PATH"
          ${builtins.readFile ../lib/memini-env.sh}

          export OPENTODO_API_TOKEN="$(cat "${config.sops.secrets."opentodo/token".path}")"
          exec ${lib.getExe llm-agents-nix.claude-code} "$@"
        ''
        // {
          version = lib.getVersion llm-agents-nix.claude-code;
        };
    in
    {
      homeModules.island.features.ai.claudeCode.statusLine =
        "${pkgs.claude-usage-bar}/bin/claude-usage-bar";

      programs.claude-code = {
        enable = true;
        package = claudeWrapped;
        configDir = "${config.xdg.configHome}/claude";

        context = ''
          ${builtins.readFile ../lib/common-context.md}
        '';

        settings = {
          theme = "dark";
          verbose = true;
          includeCoAuthoredBy = false;
          autoMemoryEnabled = false;
          remoteControlAtStartup = false;

          hooks.PreToolUse = [
            {
              matcher = "Bash|WebFetch|WebSearch|Write|Edit";
              hooks = [
                {
                  type = "command";
                  command = "${lib.getExe pkgs.pii-egress-guard} ${config.sops.secrets."krezh/email".path}";
                }
              ];
            }
          ];

          statusLine = {
            command = lib.mkDefault "${pkgs.claude-usage-bar}/bin/claude-usage-bar";
            type = "command";
          };

          enabledPlugins = {
            "gopls-lsp@claude-plugins-official" = true;
            "rust-analyzer-lsp@claude-plugins-official" = true;
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
