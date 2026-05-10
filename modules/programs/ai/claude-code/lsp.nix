_: {
  flake.modules.homeManager.ai =
    { pkgs, lib, ... }:
    {
      programs.claude-code.lspServers = {
        nix = {
          command = lib.getExe pkgs.nixd;
          extensionToLanguage.".nix" = "nix";
        };
        go = {
          command = lib.getExe pkgs.gopls;
          args = [ "serve" ];
          extensionToLanguage.".go" = "go";
        };
        rust = {
          command = lib.getExe pkgs.rust-analyzer;
          args = [ ];
          extensionToLanguage.".rs" = "rust";
        };
        typescript = {
          command = lib.getExe pkgs.typescript-language-server;
          args = [ "--stdio" ];
          extensionToLanguage = {
            ".js" = "javascript";
            ".jsx" = "javascriptreact";
            ".ts" = "typescript";
            ".tsx" = "typescriptreact";
          };
        };
        bash = {
          command = lib.getExe pkgs.bash-language-server;
          args = [ "start" ];
          extensionToLanguage = {
            ".sh" = "shellscript";
            ".bash" = "shellscript";
            ".zsh" = "shellscript";
          };
        };
      };
    };
}
