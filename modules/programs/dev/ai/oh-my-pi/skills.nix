{ inputs, ... }:
{
  flake.modules.homeManager.ai = {
    home.file = {
      ".omp/agent/skills/code-comments".source = inputs.code-comments.outPath;
      ".omp/agent/skills/herdr".source = "${inputs.herdr}/skills/herdr";
    };
  };
}
