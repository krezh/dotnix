{
  flake.modules.homeManager.browsers =
    { config, ... }:
    {
      programs.zen-browser.profiles.${config.home.username}.liveFolders = {
        "Pull Requests" = {
          id = "1793552275950-88";
          kind = "github:pull-requests";
          workspace = "fe544695-94f7-436f-942a-101c90bd83c5";
          folderIcon = "chrome://browser/skin/zen-icons/selectable/logo-github.svg";
          collapsed = false;
          fetchInterval = 900000;
          github.reviewRequested = true;
        };
      };
    };
}
