{ inputs, ... }:
{
  flake.modules.homeManager.ai = {
    programs.claude-code = {
      settings = { };

      skills = {
        herdr = "${inputs.herdr}/skills/herdr/SKILL.md";
      };
    };
  };
}
