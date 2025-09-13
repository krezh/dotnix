{ ... }:
{
  programs.direnv = {
    enable = true;
    nix-direnv.enable = true;
    #nix-direnv.package = pkgs.lixPackageSets.stable.nix-direnv; # TODO Broken atm https://git.lix.systems/lix-project/lix/issues/980
    silent = true;
  };
}
