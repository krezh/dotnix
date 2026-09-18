{
  flake.modules.homeManager.ai = {
    programs.opencode.settings = {
      # OpenCode uses per-tool permissions with states: "allow", "ask", or "deny"
      # For bash: the last matching rule takes precedence, so wildcards should be listed first
      permission = {
        # Core tool permissions
        edit = "ask";
        read = "allow";
        glob = "allow";
        grep = "allow";
        list = "allow";
        task = "allow";
        todowrite = "allow";
        todoread = "allow";
        question = "allow";
        webfetch = "allow";

        # Bash command permissions with pattern matching
        bash = {
          # Wildcard first (lowest precedence)
          "*" = "ask";

          "sudo *" = "deny";

          # Allow safe git commands
          "git add *" = "allow";
          "git status" = "allow";
          "git log *" = "allow";
          "git diff *" = "allow";
          "git show *" = "allow";
          "git branch *" = "allow";
          "git remote *" = "allow";

          # Allow safe Nix commands
          "nix *" = "allow";
          "nh search *" = "allow";
          "nh os build *" = "allow";

          # Allow safe programming tools
          "cargo *" = "allow";
          "go *" = "allow";

          # Allow safe file system operations
          "ls *" = "allow";
          "find *" = "allow";
          "grep *" = "allow";
          "rg *" = "allow";
          "cat *" = "allow";
          "head *" = "allow";
          "tail *" = "allow";
          "mkdir *" = "allow";
          "chmod *" = "allow";

          # Allow safe system info commands
          "systemctl list-units *" = "allow";
          "systemctl list-timers *" = "allow";
          "systemctl status *" = "allow";
          "journalctl *" = "allow";
          "dmesg *" = "allow";
          "env" = "allow";
          "opencode --version" = "allow";

          # Allow audio system commands
          "pactl list *" = "allow";
          "pw-top" = "allow";

          # Impeccable skill launcher (read-only context/analysis verbs)
          "*/skills/impeccable/scripts/impeccable *" = "allow";
        };
      };
    };
  };
}
