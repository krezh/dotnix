{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    let
      sharedSkills = {
        code-comments = inputs.code-comments.outPath;
        herdr = "${inputs.herdr}/skills/herdr/SKILL.md";
      };
    in
    {
      programs.claude-code.skills = sharedSkills;
      programs.codex.skills = sharedSkills;
    };
}
