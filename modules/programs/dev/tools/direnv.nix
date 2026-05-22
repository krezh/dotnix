{
  flake.modules.homeManager.dev-tools =
    { pkgs, ... }:
    {
      programs.direnv = {
        enable = true;
        nix-direnv.enable = true;
        nix-direnv.package = pkgs.lixPackageSets.latest.nix-direnv;
        silent = true;
      };
    };
}
