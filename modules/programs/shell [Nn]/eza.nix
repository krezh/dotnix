{
  flake.modules.homeManager.shell = {
    programs.eza = {
      enable = true;
      enableFishIntegration = true;
      enableBashIntegration = true;
      git = true;
      icons = "auto";
    };
  };
}
