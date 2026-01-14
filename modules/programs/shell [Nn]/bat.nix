{
  flake.modules.homeManager.shell = {
    programs.bat = {
      enable = true;
      config = {
        style = "auto";
        paging = "auto";
      };
    };
  };
}
