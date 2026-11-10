{
  flake.modules.homeManager.ai =
    { pkgs, ... }:
    {
      home.packages = [ pkgs.herdr-app ];
    };
}
