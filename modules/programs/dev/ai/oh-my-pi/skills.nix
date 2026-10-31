{ inputs, ... }:
{
  flake.modules.homeManager.ai = {
    xdg.configFile = {
      "omp/agent/skills/code-comments".source = inputs.code-comments.outPath;
      "omp/agent/skills/fluid-interfaces".source = "${inputs.agent-skills}/skills/fluid-interfaces";
    };
  };
}
