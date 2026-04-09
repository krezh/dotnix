{ inputs, ... }:
{
  flake.modules.homeManager.editors =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    {
      programs.vscode = {
        profiles.default = {
          extensions = pkgs.nix4vscode.forVscodeVersion config.programs.vscode.package.version [
            "jnoortheen.nix-ide"
            "jeff-hykin.better-nix-syntax"
          ];
          userSettings = {
            nix = {
              enableLanguageServer = true;
              serverPath = lib.getExe pkgs.nixd;
              formatterPath = lib.getExe pkgs.nixfmt;
              serverSettings = {
                nixd = {
                  nixpkgs.expr = "import ${inputs.nixpkgs} { }";
                  options = {
                    nixos.expr = ''
                      (let
                        pkgs = import ${inputs.nixpkgs} { };
                      in (pkgs.lib.evalModules {
                        modules = (import ${inputs.nixpkgs}/nixos/modules/module-list.nix) ++ [
                          ({...}: { nixpkgs.hostPlatform = "${pkgs.stdenv.hostPlatform.system}"; })
                        ];
                      })).options
                    '';
                    home-manager.expr = ''
                      (let
                        pkgs = import ${inputs.nixpkgs} { };
                        lib = import ${inputs.home-manager}/modules/lib/stdlib-extended.nix pkgs.lib;
                      in (lib.evalModules {
                        modules = (import ${inputs.home-manager}/modules/modules.nix) {
                          inherit lib pkgs;
                          check = false;
                        };
                      })).options
                    '';
                  };
                  diagnostic.suppress = [ "sema-extra-with" ];
                  hiddenLanguageServerErrors = [
                    "textDocument/definition"
                    "unknown node type for definition"
                  ];
                };
              };
            };
            "[nix]".editor.defaultFormatter = "jnoortheen.nix-ide";
          };
        };
      };
    };
}
