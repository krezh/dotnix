{ inputs, ... }:
{
  flake.modules.homeManager.editors =
    { lib, pkgs, ... }:
    {
      vscodium.extensionIds = [
        "jnoortheen.nix-ide"
        "jeff-hykin.better-nix-syntax"
      ];

      programs.vscodium = {
        profiles.default = {
          userSettings = {
            nix = {
              enableLanguageServer = true;
              serverPath = lib.getExe pkgs.nixd;
              formatterPath = lib.getExe pkgs.nixfmt;
              serverSettings = {
                nixd = rec {
                  nixpkgs.expr = "import ${inputs.nixpkgs} { }";
                  options = {
                    nixos.expr = ''
                      (let
                        pkgs = ${nixpkgs.expr};
                      in (pkgs.lib.evalModules {
                        modules = (import ${inputs.nixpkgs}/nixos/modules/module-list.nix) ++ [
                          ({...}: { nixpkgs.hostPlatform = "${pkgs.stdenv.hostPlatform.system}"; })
                        ];
                      })).options
                    '';
                    home-manager.expr = ''
                      (let
                        pkgs = ${nixpkgs.expr};
                        lib = import ${inputs.home-manager}/modules/lib/stdlib-extended.nix pkgs.lib;
                      in (lib.evalModules {
                        modules = (import ${inputs.home-manager}/modules/modules.nix) {
                          inherit lib pkgs;
                          check = false;
                        };
                      })).options
                    '';
                  };
                  diagnostic.suppress = [ ];
                  hiddenLanguageServerErrors = [ ];
                };
              };
            };
            "[nix]".editor.defaultFormatter = "jnoortheen.nix-ide";
          };
        };
      };
    };
}
