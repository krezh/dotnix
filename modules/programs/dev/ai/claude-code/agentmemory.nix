{
  flake.modules.homeManager.ai =
    { pkgs, config, ... }:
    let
      tokenPath = config.sops.secrets."agentmemory/token".path;

      mkHookWrapper =
        name:
        pkgs.writeShellScript "agentmemory-${name}" ''
          export AGENTMEMORY_URL="https://agentmemory.plexuz.xyz"
          export AGENTMEMORY_SECRET="$(cat ${tokenPath})"
          exec ${pkgs.nodejs}/bin/node ${pkgs.agentmemory}/scripts/${name}.mjs "$@"
        '';

      hook = name: [
        {
          hooks = [
            {
              type = "command";
              command = toString (mkHookWrapper name);
            }
          ];
        }
      ];
      hookWithMatcher = name: matcher: [
        {
          inherit matcher;
          hooks = [
            {
              type = "command";
              command = toString (mkHookWrapper name);
            }
          ];
        }
      ];
    in
    {
      programs.claude-code = {
        mcpServers.agentmemory = {
          type = "stdio";
          command = pkgs.writeShellScript "agentmemory-mcp-wrapper" ''
            export AGENTMEMORY_URL="https://agentmemory.plexuz.xyz"
            export AGENTMEMORY_SECRET="$(cat ${tokenPath})"
            export AGENTMEMORY_FORCE_PROXY=1
            export AGENTMEMORY_TOOLS=all
            exec ${pkgs.nodejs}/bin/node ${pkgs.agentmemory}/dist/standalone.mjs "$@"
          '';
        };

        settings.hooks = {
          SessionStart = hook "session-start";
          UserPromptSubmit = hook "prompt-submit";
          PreToolUse = hookWithMatcher "pre-tool-use" "Edit|Write|Read|Glob|Grep";
          PostToolUse = hook "post-tool-use";
          PostToolUseFailure = hook "post-tool-failure";
          PreCompact = hook "pre-compact";
          SubagentStart = hook "subagent-start";
          SubagentStop = hook "subagent-stop";
          Notification = hook "notification";
          TaskCompleted = hook "task-completed";
          Stop = hook "stop";
          SessionEnd = hook "session-end";
        };
      };
    };
}
