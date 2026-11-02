{
  perSystem =
    { pkgs, ... }:
    {
      treefmt = {
        projectRootFile = "flake.nix";
        settings = {
          global.excludes = [ "*.sops.yaml" ];
          formatter.statix.command = pkgs.lib.mkForce (
            pkgs.writeShellScriptBin "statix" ''
              for file in "$@"; do
                ${pkgs.lib.getExe pkgs.statix} fix "$file"
                ${pkgs.lib.getExe pkgs.statix} check "$file"
              done
            ''
          );
        };
        programs = {
          nixfmt = {
            enable = pkgs.lib.meta.availableOn pkgs.stdenv.buildPlatform pkgs.nixfmt.compiler;
            package = pkgs.nixfmt;
          };
          shellcheck.enable = true;
          deadnix.enable = true;
          jsonfmt.enable = true;
          gofmt.enable = true;
          terraform.enable = true;
          actionlint.enable = false;
          rustfmt.enable = true;
          statix.enable = true;
          prettier = {
            enable = true;
            settings = {
              singleQuote = false;
              tabWidth = 2;
              useTabs = false;
              bracketSpacing = true;
              bracketSameLine = true;
              singleAttributePerLine = true;
              endOfLine = "lf";
            };
          };
        };
      };
    };
}
