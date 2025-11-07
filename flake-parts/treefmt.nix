{ ... }:
{
  perSystem =
    { pkgs, ... }:
    {
      treefmt = {
        projectRootFile = "flake.nix";
        settings = {
          global.excludes = [
            "*.sops.yaml"
          ];
        };
        programs = {
          nixfmt = {
            enable = pkgs.lib.meta.availableOn pkgs.stdenv.buildPlatform pkgs.nixfmt-rfc-style.compiler;
            package = pkgs.nixfmt-rfc-style;
          };
          shellcheck.enable = true;
          deadnix.enable = true;
          jsonfmt.enable = true;
          gofmt.enable = true;
          terraform.enable = true;
          prettier = {
            enable = true;
            settings = {
              singleQuote = false;
              tabWidth = 2;
              useTabs = false;
              bracketSpacing = true;
              bracketSameLine = true;
              endOfLine = "lf";
            };
          };
        };
      };
    };
}
