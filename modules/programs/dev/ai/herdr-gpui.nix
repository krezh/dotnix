{ inputs, ... }:
{
  flake.modules.homeManager.ai =
    { pkgs, ... }:
    {
      home.packages = [
        inputs.herdr-gpui.packages.${pkgs.stdenv.hostPlatform.system}.default
      ];
    };
}
