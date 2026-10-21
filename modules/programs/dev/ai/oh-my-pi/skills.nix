{ inputs, ... }:
{
  flake.modules.homeManager.ai = {
    xdg.configFile = {
      "omp/agent/skills/code-comments".source = inputs.code-comments.outPath;
    };
  };
}
