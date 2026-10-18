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
      opencodeWrapped =
        pkgs.writeShellScriptBin "opencode" ''
          set -euo pipefail
          export PATH="${pkgs.infisical}/bin:$PATH"
          ${builtins.readFile ../lib/memini-env.sh}
          if [ -r "${config.sops.secrets."opentodo/token".path}" ]; then
            export OPENTODO_API_TOKEN="$(cat "${config.sops.secrets."opentodo/token".path}")"
          fi
          exec ${lib.getExe llm-agents-nix.opencode} "$@"
        ''
        // {
          version = lib.getVersion llm-agents-nix.opencode;
        };
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

          plugin = [
            "@eleboucher/opencode-memini"
            "@slkiser/opencode-quota"
          ];

          experimental = {
            quotaToast = {
              enabledProviders = [ "openai" ];
              formatStyle = "allWindows";
              percentDisplayMode = "remaining";
              tuiSidebarPanel.enabled = true;
              tuiCompactStatus.enabled = true;
              enableToast = true;
            };
          };
        };
      };
    };
}
