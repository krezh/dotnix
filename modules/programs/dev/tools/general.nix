{ inputs, ... }:
{
  flake.modules.homeManager.dev-tools =
    { pkgs, ... }:
    {
      home.packages = with pkgs; [
        devenv
        lefthook
        go-task
        zizmor
        shellcheck
        yaml-language-server
        inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.treefmt
        yamlfmt
      ];
    };
}
