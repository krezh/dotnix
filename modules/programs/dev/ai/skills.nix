{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    let
      sharedSkills = {
        code-comments = inputs.code-comments.outPath;
      };
    in
    {
      programs.claude-code.skills = sharedSkills;
    };
}
