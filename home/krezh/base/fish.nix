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

      curl = condDef "curlie";
    };
    shellAliases = {
      # Clear screen and scrollback
      clear = "printf '\\033[2J\\033[3J\\033[1;1H'";
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
    ];
    functions = {
      # Disable greeting
      fish_greeting = "";
    };
    interactiveShellInit = ''
      ${lib.getExe pkgs.fastfetch}
      ${lib.getExe pkgs.any-nix-shell} fish --info-right | source
    '';
  };
}
