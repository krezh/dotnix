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

      yamlFormat = pkgs.formats.yaml { };
      jsonFormat = pkgs.formats.json { };

      ompConfig = {
        modelRoles = {
          default = "openai-codex/gpt-5.6-sol:high";
          slow = "openai-codex/gpt-5.6-sol:xhigh";
          smol = "openai-codex/gpt-6-luna:low";
          plan = "@slow";
          commit = "@smol";
        };
        symbolPreset = "nerd";
        composer = {
          shape = "claude";
        };
        theme = {
          dark = "titanium";
          light = "light";
        };
        startup = {
          checkUpdate = false;
        };
        marketplace = {
          autoUpdate = "auto";
        };
        statusLine = {
          preset = "custom";
          transparent = true;
          separator = "powerline-thin";
          leftSegments = [
            "pi"
            "model"
            "usage"
            "mode"
            "collab"
            "stream"
            "path"
            "git"
            "pr"
            "context_pct"
          ];
          rightSegments = [
            "session_name"
          ];
        };
        terminal = {
          showImages = true;
          showProgress = true;
        };
        images = {
          autoResize = true;
          blockImages = false;
          describeForTextModels = true;
          questionTimeoutMs = 300000;
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

      ompConfigFile = yamlFormat.generate "omp-config.yaml" ompConfig;

      ompWrapped = pkgs.writeShellScriptBin "omp" ''
        set -euo pipefail
        export PATH="${pkgs.nodejs}/bin:${pkgs.bun}/bin:${pkgs.infisical}/bin:$PATH"
        export PI_CONFIG_DIR=".config/omp"
        export PI_CONFIG_FILES="${ompConfigFile}"
        if [ "''${1:-}" != usage ]; then
          ${builtins.readFile ../lib/memini-env.sh}
        fi
        if [ -r "${config.sops.secrets."opentodo/token".path}" ]; then
          export OPENTODO_API_TOKEN="$(cat "${config.sops.secrets."opentodo/token".path}")"
        fi
        exec ${lib.getExe llm-agents-nix.omp} "$@"
      '';

      ompPackage =
        pkgs.symlinkJoin {
          name = "omp-wrapped";
          paths = [ ompWrapped ];
          postBuild = "ln -s omp $out/bin/oh-my-pi";
          meta.mainProgram = "omp";
        }
        // {
          version = lib.getVersion llm-agents-nix.omp;
        };
    in
    {
      home.packages = [ ompPackage ];

      xdg.configFile = {
        "omp/agent/AGENTS.md".text = ''
          ${builtins.readFile ../lib/common-context.md}
          # Memory
          - Use the `memini` MCP server for all persistent cross-session memory operations.
            If memini is unavailable, proceed without persistent memory.
        '';
        "omp/agent/lsp.json".source = jsonFormat.generate "omp-lsp.json" {
          servers = {
            nil.disabled = true;
            bashls.command = lib.getExe pkgs.bash-language-server;
          };
        };
      };
    };
}
