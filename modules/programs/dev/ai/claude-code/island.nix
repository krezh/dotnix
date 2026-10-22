{
  flake.modules.homeManager.ai =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      island = config.homeModules.island or { enable = false; };
      sessions = island.features.ai.sessions;
      run = event: "${lib.getExe island.package} ai-hook ${event}";
      statusLine = pkgs.writeShellScript "claude-status-line" ''
        input=$(cat)
        printf '%s' "$input" | ${run "StatusLine"} >/dev/null 2>&1 &
        printf '%s' "$input" | ${pkgs.claude-usage-bar}/bin/claude-usage-bar
      '';
      tell = event: matcher: [
        {
          inherit matcher;
          hooks = [
            {
              type = "command";
              command = run event;
              async = true;
              timeout = 10;
            }
          ];
        }
      ];
      ask = event: matcher: {
        inherit matcher;
        hooks = [
          {
            type = "command";
            command = run event;
            # island hands a request back after an hour at most (requests::MOST_MS), even while held.
            timeout = 60 * 60 + 15;
          }
        ];
      };
    in
    lib.mkIf (island.enable && sessions.enable) {
      homeModules.island.features.ai.chat.command = lib.getExe config.programs.claude-code.package;
      programs.claude-code.settings.statusLine.command = "${statusLine}";
      programs.claude-code.settings.preferredNotifChannel = "notifications_disabled";
      programs.claude-code.settings.hooks = {
        SessionStart = tell "SessionStart" "*";
        SessionEnd = tell "SessionEnd" "*";
        UserPromptSubmit = tell "UserPromptSubmit" "*";
        Stop = tell "Stop" "*";
        Notification = tell "Notification" "*";
        PostToolUse = tell "PostToolUse" "*";
        PreToolUse = tell "ToolUse" "*" ++ [ (ask "PreToolUse" "AskUserQuestion") ];
        PermissionRequest = [ (ask "PermissionRequest" "*") ];
      };
    };
}
