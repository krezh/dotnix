{
  pkgs,
  lib,
  config,
  ...
}:
let
  inherit (lib) mkIf;
  packageNames = map (p: p.pname or p.name or null) config.home.packages;
  hasPackage = name: lib.any (x: x == name) packageNames;
  hasEza = hasPackage "eza";
  hasKubectl = hasPackage "kubectl";
  hasNeovim = config.programs.neovim.enable;
in
{
  programs.fish = {
    enable = true;
    shellAbbrs = rec {
      vim = mkIf hasNeovim "nvim";
    };
    shellAliases = {
      # Clear screen and scrollback
      clear = "printf '\\033[2J\\033[3J\\033[1;1H'";
      k = mkIf hasKubectl "kubectl";
      ls = mkIf hasEza "eza";
    };
    plugins = [
      {
        name = "puffer";
        src = pkgs.fishPlugins.puffer.src;
      }
      {
        name = "fzf-fish";
        src = pkgs.fishPlugins.fzf-fish.src;
      }
      {
        name = "autopair";
        src = pkgs.fishPlugins.autopair.src;
      }
      {
        name = "bass";
        src = pkgs.fishPlugins.bass.src;
      }
      {
        name = "forgit";
        src = pkgs.fishPlugins.forgit.src;
      }
      {
        name = "zoxide";
        src = pkgs.fetchFromGitHub {
          owner = "kidonng";
          repo = "zoxide.fish";
          rev = "bfd5947bcc7cd01beb23c6a40ca9807c174bba0e";
          sha256 = "Hq9UXB99kmbWKUVFDeJL790P8ek+xZR5LDvS+Qih+N4=";
        };
      }
    ];
    functions = {
      # Disable greeting
      fish_greeting = "";
    };
    interactiveShellInit = ''
      nitch
    '';
  };
}
