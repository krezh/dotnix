{
  flake.modules.homeManager.editors =
    { lib, pkgs, ... }:
    {
      vscodium.extensionIds = [ "rust-lang.rust-analyzer" ];

      programs.vscodium.profiles.default.userSettings = {
        rust-analyzer.server.path = lib.getExe pkgs.rust-analyzer;
      };
    };
}
