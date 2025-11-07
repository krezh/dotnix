{ ... }:
{
  programs.claude-code = {
    enable = true;
    settings = {
      hooks = {
        PostToolUse = [
          {
            hooks = [
              {
                command = "nix fmt $(jq -r '.tool_input.file_path' <<< '$CLAUDE_TOOL_INPUT')";
                type = "command";
              }
            ];
            matcher = "Edit|MultiEdit|Write";
          }
        ];
        PreToolUse = [
          {
            hooks = [
              {
                command = "echo 'Running command: $CLAUDE_TOOL_INPUT'";
                type = "command";
              }
            ];
            matcher = "Bash";
          }
        ];
      };
      includeCoAuthoredBy = false;
      model = "claude-sonnet-4-5";
      permissions = {
        allow = [
          "Bash(git diff:*)"
          "Bash(git log:*)"
          "Edit"
        ];
        ask = [
          "Bash(git push:*)"
        ];
        defaultMode = "acceptEdits";
        deny = [
          "Bash(curl:*)"
          "Read(./.env)"
          "Read(./.env.*)"
          "Read(./secrets/**)"
        ];
        disableBypassPermissionsMode = "disable";
      };
      statusLine = {
        command = "input=$(cat); echo \"[$(echo \"$input\" | jq -r '.model.display_name')] 📁 $(basename \"$(echo \"$input\" | jq -r '.workspace.current_dir')\")\"";
        padding = 0;
        type = "command";
      };
      theme = "dark";
    };
  };
}
