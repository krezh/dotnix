{ inputs, ... }:
{
  flake.modules.homeManager.dev-tools =
    { pkgs, ... }:
    {
      home.packages = with pkgs; [
        devenv
        lefthook
        zizmor
        shellcheck
        inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.treefmt
      ];
    };
}
