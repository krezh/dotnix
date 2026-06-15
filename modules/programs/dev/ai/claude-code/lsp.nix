{
  flake.modules.homeManager.ai =
    { pkgs, lib, ... }:
    {
      programs.claude-code.lspServers = {
        nix = {
          command = lib.getExe pkgs.nixd;
          extensionToLanguage.".nix" = "nix";
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
