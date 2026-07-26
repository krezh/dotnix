{ inputs, ... }:
{
  flake.modules.homeManager.shell =
    { pkgs, ... }:
    let
      undo = inputs.undo.packages.${pkgs.stdenv.hostPlatform.system}.default;
    in
    {
      home.packages = [ undo ];
      programs.fish.interactiveShellInit = ''
        source ${undo}/share/undo/undo.fish
      '';
    };
}
