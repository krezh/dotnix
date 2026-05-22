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
                    home_manager.expr = ''
                      (let
                        pkgs = ${nixpkgs.expr};
                        lib = import ${inputs.home-manager}/modules/lib/stdlib-extended.nix pkgs.lib;
                        flake = builtins.getFlake (toString ./.);
                      in (lib.evalModules {
                        modules =
                          ((import ${inputs.home-manager}/modules/modules.nix) {
                            inherit lib pkgs;
                            check = false;
                          })
                          ++ (builtins.attrValues flake.modules.homeManager)
                          ++ [
                            {
                              _module.args = {
                                inherit pkgs;
                                osConfig.system.stateVersion = "24.05";
                              };
                            }
                          ];
                      })).options
                    '';
                    flake_parts.expr = "let flake = builtins.getFlake (toString ./.); in flake.debug.options // flake.currentSystem.options";

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
