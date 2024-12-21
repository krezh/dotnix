{
  lib,
  config,
  pkgs,
  ...
}:
let
  packageNames = map (p: p.pname or p.name or null) config.home.packages;
  hasPackage = name: lib.elem name packageNames;
  createAlias = name: lib.mkIf (hasPackage name) name;
in
{
  programs.fish = {
    enable = true;
    shellAbbrs = { };
    shellAliases = {
      # Clear screen and scrollback
      clear = "printf '\\033[2J\\033[3J\\033[1;1H'";
      k = createAlias "kubectl";
      ls = createAlias "eza";
      cat = createAlias "bat";
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
    ];
    functions = {
      # Disable greeting
      fish_greeting = "";
    };
    interactiveShellInit = ''
      ${pkgs.nitch}/bin/nitch
    '';
  };
}
