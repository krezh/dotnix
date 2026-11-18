{ inputs, ... }:
{
  flake.modules.homeManager.desktop-utils =
    { pkgs, ... }:
    {
      home.packages = [ inputs.serein.packages.${pkgs.stdenv.hostPlatform.system}.default ];
    };
}
