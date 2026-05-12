{
  flake.modules.homeManager.editors =
    { pkgs, config, ... }:
    {
      programs.vscodium = {
        profiles.default = {
          extensions = pkgs.nix4vscode.forVscodeVersion config.programs.vscodium.package.version [
            "nefrob.vscode-just-syntax"
          ];
          userSettings = {
            "[just]".editor.defaultFormatter = "nefrob.vscode-just-syntax";
          };
        };
      };
    };
}
