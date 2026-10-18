{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    let
      sharedSkills = {
        code-comments = inputs.code-comments.outPath;
        herdr = "${inputs.herdr}/skills/herdr/SKILL.md";
        herdr-gpui-browser = "${inputs.herdr-gpui}/skills/herdr-gpui-browser/SKILL.md";
      };
    in
    {
      programs.claude-code.skills = sharedSkills;
    };
}
