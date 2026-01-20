{...}: {
  perSystem = {pkgs, ...}: {
    treefmt = {
      projectRootFile = "flake.nix";
      settings = {
        global.excludes = ["*.sops.yaml"];
      };
      programs = {
        alejandra = {
          enable = true;
          package = pkgs.alejandra;
        };
        shellcheck.enable = true;
        deadnix.enable = true;
        jsonfmt.enable = true;
        gofmt.enable = true;
        terraform.enable = true;
        actionlint.enable = false;
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
