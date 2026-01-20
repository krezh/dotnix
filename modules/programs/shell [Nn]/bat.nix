{
  flake.modules.homeManager.shell = {pkgs, ...}: {
    programs.bat = {
      enable = true;
      config = {
        paging = "never";
        style = "plain";
      };
      syntaxes.hyprlang = {
        src = pkgs.fetchFromGitHub {
          owner = "0x000C0A71";
          repo = "hyprlang-sublime-syntax";
          rev = "v2.0.0";
          hash = "sha256-KyiEoG5l25UNVvqDuzYVnqg2ut25Q+BDGRLw6mQGwDU=";
        };
        file = "hyprlang.sublime-syntax";
      };
    };
    programs.fish.shellAliases = {
      cat = "bat";
    };
  };
}
