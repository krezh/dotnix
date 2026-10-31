{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    let
      sharedSkills = {
        code-comments = inputs.code-comments.outPath;
        fluid-interfaces = "${inputs.agent-skills}/skills/fluid-interfaces";
      };
    in
    {
      programs.claude-code.skills = sharedSkills;
    };
}
