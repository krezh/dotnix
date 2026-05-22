{
  flake.modules.homeManager.dev-tools =
    { pkgs, ... }:
    {
      home.packages = with pkgs; [
        opentofu
        tofu-ls
      ];
    };
}
