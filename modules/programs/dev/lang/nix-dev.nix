{
  flake.modules.homeManager.nix-dev =
    { pkgs, ... }:
    {
      home.packages = with pkgs; [
        nixd
        nil
        statix
      ];
    };
}
