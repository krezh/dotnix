{ inputs, ... }:
{
  flake.modules.homeManager.shell =
    { pkgs, ... }:
    {
      home.packages = [ inputs.herdr.packages.${pkgs.stdenv.hostPlatform.system}.default ];
    };
}
