{
  flake.modules.homeManager.dev-tools =
    { pkgs, ... }:
    {
      home.packages = with pkgs; [
        yaml-language-server
        yamlfmt
        yq-go
      ];
    };
}
