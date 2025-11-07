{
  programs.bat = {
    enable = true;
    config = {
      paging = "never";
      style = "plain";
    };
  };
  programs.fish.shellAliases = {
    cat = "bat";
  };
}
