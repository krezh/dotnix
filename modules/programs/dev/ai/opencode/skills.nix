{ inputs, ... }:
{
  flake.modules.homeManager.ai = {
    # https://impeccable.style — design vocabulary skill + /impeccable command.
    # The skill's launcher fetches its matching engine binary into
    # ~/.impeccable on first use, so only the skill tree is pinned here.
    programs.opencode = {
      skills.impeccable = "${inputs.impeccable}/.opencode/skills/impeccable";
      commands.impeccable = builtins.readFile "${inputs.impeccable}/.opencode/commands/impeccable.md";
    };
  };
}
