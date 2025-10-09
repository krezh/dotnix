{
  lib,
  config,
  pkgs,
  ...
}:
let
  condDef =
    name:
    let
      packageNames = map (p: p.pname or p.name or null) config.home.packages;
      hasPackage = lib.elem name packageNames;
    in
    lib.mkIf hasPackage name;
in
{
  programs.fish = {
    enable = true;
    package = pkgs.fish;
    shellAbbrs = {
      # git
      gs = "git status";
      gc = "git commit";
      gcm = "git ci -m";
      gco = "git co";
      ga = "git add -A";
      gm = "git merge";
      gl = "git l";
      gd = "git diff";
      gb = "git b";
      gpl = "git pull";
      gp = "git push";
      gpc = "git push -u origin (git rev-parse --abbrev-ref HEAD)";
      gpf = "git push --force-with-lease";
      gbc = "git nb";

      curl = "curlie";

      # kubernetes
      k = condDef "kubectl";
    };
    shellAliases = {
      # Clear screen and scrollback
      clear = "printf '\\033[2J\\033[3J\\033[1;1H'";
      kubectl = condDef "kubecolor";
      cat = condDef "bat";
    };
    plugins = [
      {
        name = "puffer";
        src = pkgs.fishPlugins.puffer.src;
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
        name = "abbreviation-tips";
        src = pkgs.fetchFromGitHub {
          owner = "gazorby";
          repo = "fish-abbreviation-tips";
          rev = "v0.7.0";
          sha256 = "sha256-F1t81VliD+v6WEWqj1c1ehFBXzqLyumx5vV46s/FZRU=";
        };
      }
      (lib.mkIf (config.programs.tmux.enable && !config.programs.zellij.enable) {
        name = "tmux";
        src = pkgs.fetchFromGitHub {
          owner = "budimanjojo";
          repo = "tmux.fish";
          rev = "v2.0.1";
          sha256 = "sha256-ynhEhrdXQfE1dcYsSk2M2BFScNXWPh3aws0U7eDFtv4=";
        };
      })
    ];
    functions = {
      # Disable greeting
      fish_greeting = "";
    };
    interactiveShellInit = ''
      ${lib.getExe pkgs.fastfetch}
      ${lib.optionalString config.programs.tmux.enable "set fish_tmux_autostart true"}
      ${lib.getExe pkgs.any-nix-shell} fish | source
    '';
  };
}
