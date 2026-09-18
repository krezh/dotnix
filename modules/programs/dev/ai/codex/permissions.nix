{
  flake.modules.homeManager.ai = {
    programs.codex.rules.default = ''
      prefix_rule(pattern = ["git", "status"], decision = "allow")
      prefix_rule(pattern = ["git", "log"], decision = "allow")
      prefix_rule(pattern = ["git", "diff"], decision = "allow")
      prefix_rule(pattern = ["git", "show"], decision = "allow")
      prefix_rule(pattern = ["git", "branch"], decision = "allow")
      prefix_rule(pattern = ["git", "remote"], decision = "allow")
      prefix_rule(pattern = ["git", "rev-parse"], decision = "allow")
      prefix_rule(pattern = ["git", "blame"], decision = "allow")

      prefix_rule(pattern = ["jj", "log"], decision = "allow")
      prefix_rule(pattern = ["jj", "diff"], decision = "allow")
      prefix_rule(pattern = ["jj", "status"], decision = "allow")
      prefix_rule(pattern = ["jj", "st"], decision = "allow")
      prefix_rule(pattern = ["jj", "show"], decision = "allow")
      prefix_rule(pattern = ["jj", "op", "log"], decision = "allow")
      prefix_rule(pattern = ["jj", "file", "show"], decision = "allow")
      prefix_rule(pattern = ["jj", "restore"], decision = "allow")
      prefix_rule(pattern = ["jj", "describe"], decision = "allow")

      prefix_rule(pattern = ["nix"], decision = "allow")
      prefix_rule(pattern = ["cargo"], decision = "allow")
      prefix_rule(pattern = ["go"], decision = "allow")

      prefix_rule(pattern = ["ls"], decision = "allow")
      prefix_rule(pattern = ["find"], decision = "allow")

      prefix_rule(pattern = ["systemctl", "status"], decision = "allow")
      prefix_rule(pattern = ["journalctl"], decision = "allow")

      prefix_rule(pattern = ["gh", "search"], decision = "allow")
      prefix_rule(pattern = ["gh", "api"], decision = "allow")

      prefix_rule(pattern = ["kubectl", "get"], decision = "allow")
      prefix_rule(pattern = ["kubectl", "logs"], decision = "allow")
      prefix_rule(pattern = ["kubectl", "describe"], decision = "allow")

      prefix_rule(pattern = ["shellcheck"], decision = "allow")
      prefix_rule(pattern = ["zizmor"], decision = "allow")
    '';
  };
}
