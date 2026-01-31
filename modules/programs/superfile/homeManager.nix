{
  flake.modules.homeManager.superfile = {
    programs.superfile = {
      enable = true;
      settings = {
        transparent_background = true;
        theme = "catppuccin";
      };
    };
  };
}
