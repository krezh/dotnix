{ ... }: {
  programs.bat = {
    enable = true;
    config = {
      paging = "never";
      style = "plain";
    };
  };
}
