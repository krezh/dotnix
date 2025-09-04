{ pkgs, ... }:
{
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
    shellcheck = {
      enable = true;
    };
    deadnix = {
      enable = true;
    };
    jsonfmt = {
      enable = true;
    };
    yamlfmt = {
      enable = false;
      settings = {
        formatter = {
          eof_newline = true;
          include_document_start = true;
          retain_line_breaks_single = true;
          trim_trailing_whitespace = true;
        };
      };
    };
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
}
