{
  flake.modules.homeManager.ai = {
    catppuccin.gemini-cli.enable = false; # Uses IFD
    programs.gemini-cli.enable = true;
  };
}
