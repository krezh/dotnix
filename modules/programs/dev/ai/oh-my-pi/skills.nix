{ inputs, ... }:
{
  flake.modules.homeManager.ai = {
    xdg.configFile = {
      "omp/agent/skills/code-comments".source = inputs.code-comments.outPath;
      "omp/agent/skills/herdr".source = "${inputs.herdr}/skills/herdr";
      "omp/agent/skills/herdr-gpui-browser".source = "${inputs.herdr-gpui}/skills/herdr-gpui-browser";
    };
  };
}
