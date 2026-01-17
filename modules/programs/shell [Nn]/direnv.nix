{
  flake.modules.homeManager.shell = {
    programs.direnv = {
      enable = true;
      nix-direnv.enable = true;
      # nix-direnv.package = pkgs.lixPackageSets.stable.nix-direnv;
      silent = true;
    };
  };
}
