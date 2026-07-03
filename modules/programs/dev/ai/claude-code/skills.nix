{ inputs, ... }:
{
  flake.modules.homeManager.ai = {
    programs.claude-code = {
      settings = { };

      skills = {
        herdr = "${inputs.herdr}/SKILL.md";
      };
    };
  };
}
